/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#ifndef _WIN32

#include <sys/stat.h>
#ifdef __APPLE__
#include <sys/mount.h>
#include <sys/param.h>
#else
#include <sys/vfs.h>
#endif

#include "eden/fs/inodes/InodeMetadata.h"
#include "eden/fs/inodes/InodeNumber.h"
#include "eden/fs/store/ObjectFetchContext.h"
#include "eden/fs/utils/PathFuncs.h"

namespace folly {
template <class T>
class Future;
}

namespace facebook::eden {

class EdenStats;
class Clock;

class NfsDispatcher {
 public:
  explicit NfsDispatcher(EdenStats* stats, const Clock& clock)
      : stats_(stats), clock_(clock) {}

  virtual ~NfsDispatcher() {}

  EdenStats* getStats() const {
    return stats_;
  }

  const Clock& getClock() const {
    return clock_;
  }

  /**
   * Get file attribute for the passed in InodeNumber.
   */
  virtual folly::Future<struct stat> getattr(
      InodeNumber ino,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the setattr method.
   */
  struct SetattrRes {
    /** Attributes of the file prior to changing its attributes */
    std::optional<struct stat> preStat;
    /** Attributes of the file after changing its attributes */
    std::optional<struct stat> postStat;
  };

  /**
   * Change the attributes of the file referenced by the InodeNumber ino.
   *
   * See comment on the create method for the meaning of the returned pre and
   * post stat.
   */
  virtual folly::Future<SetattrRes> setattr(
      InodeNumber ino,
      DesiredMetadata desired,
      ObjectFetchContext& context) = 0;

  /**
   * Racily obtain the parent directory of the passed in directory.
   *
   * Can be used to handle a ".." filename.
   */
  virtual folly::Future<InodeNumber> getParent(
      InodeNumber ino,
      ObjectFetchContext& context) = 0;

  /**
   * Find the given file in the passed in directory. It's InodeNumber and
   * attributes are returned.
   */
  virtual folly::Future<std::tuple<InodeNumber, struct stat>>
  lookup(InodeNumber dir, PathComponent name, ObjectFetchContext& context) = 0;

  /**
   * For a symlink, return its destination, fail otherwise.
   */
  virtual folly::Future<std::string> readlink(
      InodeNumber ino,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the read method.
   */
  struct ReadRes {
    /** Data successfully read */
    std::unique_ptr<folly::IOBuf> data;
    /** Has the read reached the end of file */
    bool isEof;
  };

  /**
   * Read data from the file referenced by the InodeNumber ino.
   */
  virtual folly::Future<ReadRes> read(
      InodeNumber ino,
      size_t size,
      off_t offset,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the write method.
   */
  struct WriteRes {
    /** Number of bytes written */
    size_t written;

    /** Attributes of the directory prior to creating the file */
    std::optional<struct stat> preStat;
    /** Attributes of the directory after creating the file */
    std::optional<struct stat> postStat;
  };

  /**
   * Write data at offset to the file referenced by the InodeNumber ino.
   *
   * See the comment on the create method below for the meaning of the returned
   * pre and post stat.
   */
  virtual folly::Future<WriteRes> write(
      InodeNumber ino,
      std::unique_ptr<folly::IOBuf> data,
      off_t offset,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the create method.
   */
  struct CreateRes {
    /** InodeNumber of the created file */
    InodeNumber ino;
    /** Attributes of the created file */
    struct stat stat;

    /** Attributes of the directory prior to creating the file */
    std::optional<struct stat> preDirStat;
    /** Attributes of the directory after creating the file */
    std::optional<struct stat> postDirStat;
  };

  /**
   * Create a regular file in the directory referenced by the InodeNumber dir.
   *
   * Both the pre and post stat for that directory needs to be collected in an
   * atomic manner: no other operation on the directory needs to be allowed in
   * between them. This is to ensure that the NFS client can properly detect if
   * its cache needs to be invalidated. Setting them both to std::nullopt is an
   * acceptable approach if the stat cannot be collected atomically.
   */
  virtual folly::Future<CreateRes> create(
      InodeNumber dir,
      PathComponent name,
      mode_t mode,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the mkdir method.
   */
  struct MkdirRes {
    /** InodeNumber of the created directory */
    InodeNumber ino;
    /** Attributes of the created directory */
    struct stat stat;

    /** Attributes of the directory prior to creating the subdirectory */
    std::optional<struct stat> preDirStat;
    /** Attributes of the directory after creating the subdirectory */
    std::optional<struct stat> postDirStat;
  };

  /**
   * Create a subdirectory in the directory referenced by the InodeNumber dir.
   *
   * For the pre and post dir stat, refer to the documentation of the create
   * method above.
   */
  virtual folly::Future<MkdirRes> mkdir(
      InodeNumber dir,
      PathComponent name,
      mode_t mode,
      ObjectFetchContext& context) = 0;

  /**
   * Return value of the unlink method.
   */
  struct UnlinkRes {
    /** Attributes of the directory prior to removing the file */
    std::optional<struct stat> preDirStat;
    /** Attributes of the directory after removing the file */
    std::optional<struct stat> postDirStat;
  };

  /**
   * Remove the file/directory name from the directory referenced by the
   * InodeNumber dir.
   *
   * For the pre and post dir stat, refer to the documentation of the create
   * method above.
   */
  virtual folly::Future<UnlinkRes>
  unlink(InodeNumber dir, PathComponent name, ObjectFetchContext& context) = 0;

  struct RenameRes {
    /** Attributes of the from directory prior to renaming the file. */
    std::optional<struct stat> fromPreDirStat;
    /** Attributes of the from directory after renaming the file. */
    std::optional<struct stat> fromPostDirStat;
    /** Attributes of the to directory prior to renaming the file. */
    std::optional<struct stat> toPreDirStat;
    /** Attributes of the to directory after renaming the file. */
    std::optional<struct stat> toPostDirStat;
  };

  /**
   * Rename a file/directory from the directory referenced by fromIno to the
   * directory referenced by toIno. The file/directory fromName will be renamed
   * onto toName.
   *
   * Fro the pre and post dir stat, refer to the documentation of the create
   * method above.
   */
  virtual folly::Future<RenameRes> rename(
      InodeNumber fromIno,
      PathComponent fromName,
      InodeNumber toIno,
      PathComponent toName,
      ObjectFetchContext& context) = 0;

  virtual folly::Future<struct statfs> statfs(
      InodeNumber dir,
      ObjectFetchContext& context) = 0;

 private:
  EdenStats* stats_{nullptr};
  const Clock& clock_;
};

} // namespace facebook::eden

#endif
