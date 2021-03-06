/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use scuba_ext::MononokeScubaSampleBuilder;

use context::CoreContext;
use mononoke_types::RepositoryId;

use crate::types::{IdDagVersion, IdMapVersion, SegmentedChangelogVersion};

const SCUBA_TABLE: &str = "segmented_changelog_version";

pub fn log_new_idmap_version(
    ctx: &CoreContext,
    repo_id: RepositoryId,
    idmap_version: IdMapVersion,
) {
    MononokeScubaSampleBuilder::new(ctx.fb, SCUBA_TABLE)
        .add_common_server_data()
        .add("type", "idmap")
        .add("repo_id", repo_id.id())
        .add("idmap_version", idmap_version.0)
        .log(); // note that logging may fail
}

pub fn log_new_iddag_version(
    ctx: &CoreContext,
    repo_id: RepositoryId,
    iddag_version: IdDagVersion,
) {
    MononokeScubaSampleBuilder::new(ctx.fb, SCUBA_TABLE)
        .add_common_server_data()
        .add("type", "iddag")
        .add("repo_id", repo_id.id())
        .add("iddag_version", format!("{}", iddag_version.0))
        .log(); // note that logging may fail
}

pub fn log_new_segmented_changelog_version(
    ctx: &CoreContext,
    repo_id: RepositoryId,
    sc_version: SegmentedChangelogVersion,
) {
    slog::info!(
        ctx.logger(),
        "repo {}: segmented changelog version saved, idmap_version: {}, iddag_version: {}",
        repo_id,
        sc_version.idmap_version,
        sc_version.iddag_version,
    );
    MononokeScubaSampleBuilder::new(ctx.fb, SCUBA_TABLE)
        .add_common_server_data()
        .add("type", "segmented_changelog")
        .add("repo_id", repo_id.id())
        .add("idmap_version", sc_version.idmap_version.0)
        .add("iddag_version", format!("{}", sc_version.iddag_version.0))
        .log(); // note that logging may fail
}
