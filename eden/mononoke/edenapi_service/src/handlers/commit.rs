/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::{Context, Error};
use futures::{stream, Stream, StreamExt, TryStreamExt};
use gotham::state::{FromState, State};
use gotham_derive::{StateData, StaticResponseExtender};
use serde::Deserialize;

use edenapi_types::{
    wire::{WireCommitHashToLocationRequestBatch, WireCommitLocationToHashRequestBatch},
    CommitHashToLocationResponse, CommitLocationToHashRequest, CommitLocationToHashResponse,
    CommitRevlogData, CommitRevlogDataRequest, ToWire,
};
use gotham_ext::{error::HttpError, response::TryIntoResponse};
use mercurial_types::HgChangesetId;
use mononoke_api_hg::HgRepoContext;
use types::HgId;

use crate::context::ServerContext;
use crate::errors::ErrorKind;
use crate::middleware::RequestContext;
use crate::utils::{cbor_stream, get_repo, parse_cbor_request, parse_wire_request};

use super::{EdenApiMethod, HandlerInfo};

/// XXX: This number was chosen arbitrarily.
const MAX_CONCURRENT_FETCHES_PER_REQUEST: usize = 100;
const HASH_TO_LOCATION_BATCH_SIZE: usize = 100;

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
pub struct LocationToHashParams {
    repo: String,
}

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
pub struct HashToLocationParams {
    repo: String,
}

#[derive(Debug, Deserialize, StateData, StaticResponseExtender)]
pub struct RevlogDataParams {
    repo: String,
}

pub async fn location_to_hash(state: &mut State) -> Result<impl TryIntoResponse, HttpError> {
    let params = LocationToHashParams::take_from(state);

    state.put(HandlerInfo::new(
        &params.repo,
        EdenApiMethod::CommitLocationToHash,
    ));

    let sctx = ServerContext::borrow_from(state);
    let rctx = RequestContext::borrow_from(state).clone();

    let hg_repo_ctx = get_repo(&sctx, &rctx, &params.repo, None).await?;

    let batch = parse_wire_request::<WireCommitLocationToHashRequestBatch>(state).await?;
    let hgid_list = batch
        .requests
        .into_iter()
        .map(move |location| translate_location(hg_repo_ctx.clone(), location));
    let response = stream::iter(hgid_list)
        .buffer_unordered(MAX_CONCURRENT_FETCHES_PER_REQUEST)
        .map_ok(|response| response.to_wire());
    Ok(cbor_stream(rctx, response))
}

pub async fn hash_to_location(state: &mut State) -> Result<impl TryIntoResponse, HttpError> {
    async fn hash_to_location_chunk(
        hg_repo_ctx: HgRepoContext,
        client_head: HgChangesetId,
        hg_cs_ids: Vec<HgChangesetId>,
    ) -> Result<impl Stream<Item = Result<CommitHashToLocationResponse, Error>>, Error> {
        let hgcsid_to_location = hg_repo_ctx
            .many_changeset_ids_to_locations(client_head, hg_cs_ids.clone())
            .await?;
        let responses = hgcsid_to_location.into_iter().map(|(hgcsid, location)| {
            let response = CommitHashToLocationResponse {
                hgid: hgcsid.into(),
                location: location.map_descendant(|x| x.into()),
            };
            Ok(response)
        });
        Ok(stream::iter(responses))
    }

    let params = HashToLocationParams::take_from(state);

    state.put(HandlerInfo::new(
        &params.repo,
        EdenApiMethod::CommitHashToLocation,
    ));

    let sctx = ServerContext::borrow_from(state);
    let rctx = RequestContext::borrow_from(state).clone();

    let hg_repo_ctx = get_repo(&sctx, &rctx, &params.repo, None).await?;

    let batch = parse_wire_request::<WireCommitHashToLocationRequestBatch>(state).await?;
    let client_head = batch.client_head.into();

    let response = stream::iter(batch.hgids)
        .chunks(HASH_TO_LOCATION_BATCH_SIZE)
        .map(|chunk| chunk.into_iter().map(|x| x.into()).collect::<Vec<_>>())
        .map({
            let ctx = hg_repo_ctx.clone();
            move |chunk| hash_to_location_chunk(ctx.clone(), client_head, chunk)
        })
        .buffer_unordered(3)
        .try_flatten()
        .map_ok(|response| response.to_wire());
    Ok(cbor_stream(rctx, response))
}

pub async fn revlog_data(state: &mut State) -> Result<impl TryIntoResponse, HttpError> {
    let params = RevlogDataParams::take_from(state);

    state.put(HandlerInfo::new(
        &params.repo,
        EdenApiMethod::CommitRevlogData,
    ));

    let sctx = ServerContext::borrow_from(state);
    let rctx = RequestContext::borrow_from(state).clone();

    let hg_repo_ctx = get_repo(&sctx, &rctx, &params.repo, None).await?;

    let request: CommitRevlogDataRequest = parse_cbor_request(state).await?;
    let revlog_commits = request
        .hgids
        .into_iter()
        .map(move |hg_id| commit_revlog_data(hg_repo_ctx.clone(), hg_id));
    let response =
        stream::iter(revlog_commits).buffer_unordered(MAX_CONCURRENT_FETCHES_PER_REQUEST);
    Ok(cbor_stream(rctx, response))
}

async fn translate_location(
    hg_repo_ctx: HgRepoContext,
    request: CommitLocationToHashRequest,
) -> Result<CommitLocationToHashResponse, Error> {
    let location = request.location.map_descendant(|x| x.into());
    let ancestors: Vec<HgChangesetId> = hg_repo_ctx
        .location_to_hg_changeset_id(location, request.count)
        .await
        .context(ErrorKind::CommitLocationToHashRequestFailed)?;
    let hgids = ancestors.into_iter().map(|x| x.into()).collect();
    let answer = CommitLocationToHashResponse {
        location: request.location,
        count: request.count,
        hgids,
    };
    Ok(answer)
}

async fn commit_revlog_data(
    hg_repo_ctx: HgRepoContext,
    hg_id: HgId,
) -> Result<CommitRevlogData, Error> {
    let bytes = hg_repo_ctx
        .revlog_commit_data(hg_id.into())
        .await
        .context(ErrorKind::CommitRevlogDataRequestFailed)?
        .ok_or_else(|| ErrorKind::HgIdNotFound(hg_id))?;
    let answer = CommitRevlogData::new(hg_id, bytes);
    Ok(answer)
}
