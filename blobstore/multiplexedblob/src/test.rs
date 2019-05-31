// Copyright (c) 2018-present, Facebook, Inc.
// All Rights Reserved.
//
// This software may be used and distributed according to the terms of the
// GNU General Public License version 2 or any later version.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

use async_unit;
use failure_ext::{err_msg, Error};
use futures::future::{Future, IntoFuture};
use futures::sync::oneshot;
use futures::Async;
use futures_ext::{BoxFuture, FutureExt};
use lock_ext::LockExt;

use blobstore::Blobstore;
use blobstore_sync_queue::{BlobstoreSyncQueue, SqlBlobstoreSyncQueue, SqlConstructors};
use context::CoreContext;
use metaconfig_types::BlobstoreId;
use mononoke_types::{BlobstoreBytes, RepositoryId};

use crate::base::{MultiplexedBlobstoreBase, MultiplexedBlobstorePutHandler};
use crate::queue::MultiplexedBlobstore;

pub struct TickBlobstore {
    pub storage: Arc<Mutex<HashMap<String, BlobstoreBytes>>>,
    // queue of pending operations
    queue: Arc<Mutex<VecDeque<oneshot::Sender<Option<String>>>>>,
}

impl fmt::Debug for TickBlobstore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TickBlobstore")
            .field("storage", &self.storage)
            .field("pending", &self.queue.with(|q| q.len()))
            .finish()
    }
}

impl TickBlobstore {
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
            queue: Default::default(),
        }
    }
    pub fn tick(&self, error: Option<&str>) {
        let mut queue = self.queue.lock().unwrap();
        for send in queue.drain(..) {
            send.send(error.map(String::from)).unwrap();
        }
    }
    pub fn on_tick(&self) -> impl Future<Item = (), Error = Error> {
        let (send, recv) = oneshot::channel();
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(send);
        recv.map_err(Error::from).and_then(|error| match error {
            None => Ok(()),
            Some(error) => Err(err_msg(error)),
        })
    }
}

impl Blobstore for TickBlobstore {
    fn get(&self, _ctx: CoreContext, key: String) -> BoxFuture<Option<BlobstoreBytes>, Error> {
        let storage = self.storage.clone();
        self.on_tick()
            .map(move |_| storage.with(|s| s.get(&key).cloned()))
            .boxify()
    }
    fn put(&self, _ctx: CoreContext, key: String, value: BlobstoreBytes) -> BoxFuture<(), Error> {
        let storage = self.storage.clone();
        self.on_tick()
            .map(move |_| {
                storage.with(|s| {
                    s.insert(key, value);
                })
            })
            .boxify()
    }
}

struct LogHandler {
    pub log: Arc<Mutex<Vec<(BlobstoreId, String)>>>,
}

impl LogHandler {
    fn new() -> Self {
        Self {
            log: Default::default(),
        }
    }
    fn clear(&self) {
        self.log.with(|log| log.clear())
    }
}

impl MultiplexedBlobstorePutHandler for LogHandler {
    fn on_put(
        &self,
        _ctx: CoreContext,
        blobstore_id: BlobstoreId,
        key: String,
    ) -> BoxFuture<(), Error> {
        self.log.with(move |log| log.push((blobstore_id, key)));
        Ok(()).into_future().boxify()
    }
}

fn make_value(value: &str) -> BlobstoreBytes {
    BlobstoreBytes::from_bytes(value.as_bytes())
}

#[test]
fn base() {
    async_unit::tokio_unit_test(|| {
        let bs0 = Arc::new(TickBlobstore::new());
        let bs1 = Arc::new(TickBlobstore::new());
        let log = Arc::new(LogHandler::new());
        let bs = MultiplexedBlobstoreBase::new(
            vec![
                (BlobstoreId::new(0), bs0.clone()),
                (BlobstoreId::new(1), bs1.clone()),
            ],
            log.clone(),
            None,
        );
        let ctx = CoreContext::test_mock();

        // succeed as soon as first blobstore succeeded
        {
            let v0 = make_value("v0");
            let k0 = String::from("k0");

            let mut put_fut = bs.put(ctx.clone(), k0.clone(), v0.clone());
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(None);
            put_fut.wait().unwrap();
            assert_eq!(bs0.storage.with(|s| s.get(&k0).cloned()), Some(v0.clone()));
            assert!(bs1.storage.with(|s| s.is_empty()));
            bs1.tick(Some("bs1 failed"));
            assert!(log
                .log
                .with(|log| log == &vec![(BlobstoreId::new(0), k0.clone())]));

            // should succeed as it is stored in bs1
            let mut get_fut = bs.get(ctx.clone(), k0);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(None);
            bs1.tick(None);
            assert_eq!(get_fut.wait().unwrap(), Some(v0));
            assert!(bs1.storage.with(|s| s.is_empty()));

            log.clear();
        }

        // wait for second if first one failed
        {
            let v1 = make_value("v1");
            let k1 = String::from("k1");

            let mut put_fut = bs.put(ctx.clone(), k1.clone(), v1.clone());
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(Some("bs0 failed"));
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs1.tick(None);
            put_fut.wait().unwrap();
            assert!(bs0.storage.with(|s| s.get(&k1).is_none()));
            assert_eq!(bs1.storage.with(|s| s.get(&k1).cloned()), Some(v1.clone()));
            assert!(log
                .log
                .with(|log| log == &vec![(BlobstoreId::new(1), k1.clone())]));

            let mut get_fut = bs.get(ctx.clone(), k1.clone());
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(None);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);
            bs1.tick(None);
            assert_eq!(get_fut.wait().unwrap(), Some(v1));
            assert!(bs0.storage.with(|s| s.get(&k1).is_none()));

            log.clear();
        }

        // both fail => whole put fail
        {
            let k2 = String::from("k2");
            let v2 = make_value("v2");

            let mut put_fut = bs.put(ctx.clone(), k2.clone(), v2.clone());
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(Some("bs0 failed"));
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs1.tick(Some("bs1 failed"));
            assert!(put_fut.wait().is_err());
        }

        // get: Error + None -> Error
        {
            let k3 = String::from("k3");
            let mut get_fut = bs.get(ctx.clone(), k3);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs0.tick(Some("bs0 failed"));
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs1.tick(None);
            assert!(get_fut.wait().is_err());
        }

        // get: None + None -> None
        {
            let k3 = String::from("k3");
            let mut get_fut = bs.get(ctx.clone(), k3);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs0.tick(None);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs1.tick(None);
            assert_eq!(get_fut.wait().unwrap(), None);
        }

        // both put succeed
        {
            let k4 = String::from("k4");
            let v4 = make_value("v4");
            log.clear();

            let mut put_fut = bs.put(ctx.clone(), k4.clone(), v4.clone());
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(None);
            put_fut.wait().unwrap();
            assert_eq!(bs0.storage.with(|s| s.get(&k4).cloned()), Some(v4.clone()));
            bs1.tick(None);
            while log.log.with(|log| log.len() != 2) {}
            assert_eq!(bs1.storage.with(|s| s.get(&k4).cloned()), Some(v4.clone()));
        }
    });
}

#[test]
fn multiplexed() {
    async_unit::tokio_unit_test(|| {
        let repoid = RepositoryId::new(0);
        let ctx = CoreContext::test_mock();
        let queue = Arc::new(SqlBlobstoreSyncQueue::with_sqlite_in_memory().unwrap());

        let bid0 = BlobstoreId::new(0);
        let bs0 = Arc::new(TickBlobstore::new());
        let bid1 = BlobstoreId::new(1);
        let bs1 = Arc::new(TickBlobstore::new());
        let bs = MultiplexedBlobstore::new(
            repoid,
            vec![(bid0, bs0.clone()), (bid1, bs1.clone())],
            queue.clone(),
            None,
        );

        // non-existing key when one blobstore failing
        {
            let k0 = String::from("k0");

            let mut get_fut = bs.get(ctx.clone(), k0.clone());
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs0.tick(None);
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);

            bs1.tick(Some("bs1 failed"));
            assert_eq!(get_fut.wait().unwrap(), None);
        }

        // only replica containing key failed
        {
            let k1 = String::from("k1");
            let v1 = make_value("v1");

            let mut put_fut = bs.put(ctx.clone(), k1.clone(), v1.clone());
            assert_eq!(put_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(None);
            bs1.tick(Some("bs1 failed"));
            put_fut.wait().unwrap();

            match queue
                .get(ctx.clone(), repoid, k1.clone())
                .wait()
                .unwrap()
                .as_slice()
            {
                [entry] => assert_eq!(entry.blobstore_id, bid0),
                _ => panic!("only one entry expected"),
            }

            let mut get_fut = bs.get(ctx.clone(), k1.clone());
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(Some("bs0 failed"));
            bs1.tick(None);
            assert!(get_fut.wait().is_err());
        }

        // both replicas fail
        {
            let k2 = String::from("k2");

            let mut get_fut = bs.get(ctx.clone(), k2.clone());
            assert_eq!(get_fut.poll().unwrap(), Async::NotReady);
            bs0.tick(Some("bs0 failed"));
            bs1.tick(Some("bs1 failed"));
            assert!(get_fut.wait().is_err());
        }
    });
}
