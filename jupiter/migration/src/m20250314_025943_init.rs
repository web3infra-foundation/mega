use extension::postgres::Type;
use sea_orm_migration::{
    prelude::*,
    schema::*,
    sea_orm::{EnumIter, Iterable},
};

use crate::pk_bigint;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(MergeStatusEnum)
                    .values(MergeStatus::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(StorageTypeEnum)
                    .values(StorageType::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(RefTypeEnum)
                    .values(RefType::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                Type::create()
                    .as_enum(ConvTypeEnum)
                    .values(ConvType::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaCommit::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaCommit::Id))
                    .col(string(MegaCommit::CommitId))
                    .col(string(MegaCommit::Tree))
                    .col(json(MegaCommit::ParentsId))
                    .col(text_null(MegaCommit::Author))
                    .col(text_null(MegaCommit::Committer))
                    .col(text_null(MegaCommit::Content))
                    .col(date_time(MegaCommit::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-mega_commit_commit_id")
                    .unique()
                    .table(MegaCommit::Table)
                    .col(MegaCommit::CommitId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaTree::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaTree::Id))
                    .col(string(MegaTree::TreeId))
                    .col(binary(MegaTree::SubTrees))
                    .col(integer(MegaTree::Size))
                    .col(string(MegaTree::CommitId))
                    .col(date_time(MegaTree::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mt_git_id")
                    .unique()
                    .table(MegaTree::Table)
                    .col(MegaTree::TreeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaBlob::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaBlob::Id))
                    .col(string(MegaBlob::BlobId))
                    .col(string(MegaBlob::CommitId))
                    .col(text(MegaBlob::Name))
                    .col(integer(MegaBlob::Size))
                    .col(date_time(MegaBlob::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mb_git_id")
                    .unique()
                    .table(MegaBlob::Table)
                    .col(MegaBlob::BlobId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaTag::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaTag::Id))
                    .col(string(MegaTag::TagId))
                    .col(string(MegaTag::ObjectId))
                    .col(string(MegaTag::ObjectType))
                    .col(text(MegaTag::TagName))
                    .col(text(MegaTag::Tagger))
                    .col(text(MegaTag::Message))
                    .col(date_time(MegaTag::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mtag_tag_id")
                    .unique()
                    .table(MegaTag::Table)
                    .col(MegaTag::TagId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaMr::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaMr::Id))
                    .col(string(MegaMr::Link))
                    .col(text(MegaMr::Title))
                    .col(date_time_null(MegaMr::MergeDate))
                    .col(enumeration(
                        MegaMr::Status,
                        Alias::new("merge_status_enum"),
                        MergeStatus::iter(),
                    ))
                    .col(text(MegaMr::Path))
                    .col(string(MegaMr::FromHash))
                    .col(string(MegaMr::ToHash))
                    .col(date_time(MegaMr::CreatedAt))
                    .col(date_time(MegaMr::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_mr_path")
                    .unique()
                    .table(MegaMr::Table)
                    .col(MegaMr::Path)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaConversation::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaConversation::Id))
                    .col(string(MegaConversation::Link))
                    .col(big_integer(MegaConversation::UserId))
                    .col(enumeration(
                        MegaConversation::ConvType,
                        Alias::new("conv_type_enum"),
                        ConvType::iter(),
                    ))
                    .col(text_null(MegaConversation::Comment))
                    .col(date_time(MegaConversation::CreatedAt))
                    .col(date_time(MegaConversation::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_conversation")
                    .unique()
                    .table(MegaConversation::Table)
                    .col(MegaConversation::Link)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaIssue::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaIssue::Id))
                    .col(string(MegaIssue::Link))
                    .col(string(MegaIssue::Title))
                    .col(big_integer(MegaIssue::Owner))
                    .col(string(MegaIssue::Status))
                    .col(date_time(MegaIssue::CreatedAt))
                    .col(date_time(MegaIssue::UpdatedAt))
                    .col(date_time_null(MegaIssue::ClosedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_issue")
                    .unique()
                    .table(MegaIssue::Table)
                    .col(MegaIssue::Link)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MegaRefs::Table)
                    .if_not_exists()
                    .col(pk_bigint(MegaRefs::Id))
                    .col(text(MegaRefs::Path))
                    .col(text(MegaRefs::RefName))
                    .col(string(MegaRefs::RefCommitHash))
                    .col(string(MegaRefs::RefTreeHash))
                    .col(date_time(MegaRefs::CreatedAt))
                    .col(date_time(MegaRefs::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_mref_path")
                    .unique()
                    .table(MegaRefs::Table)
                    .col(MegaRefs::Path)
                    .col(MegaRefs::RefName)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ImportRefs::Table)
                    .if_not_exists()
                    .col(pk_bigint(ImportRefs::Id))
                    .col(big_integer(ImportRefs::RepoId))
                    .col(text(ImportRefs::RefName))
                    .col(string(ImportRefs::RefGitId))
                    .col(enumeration(
                        ImportRefs::RefType,
                        Alias::new("ref_type_enum"),
                        RefType::iter(),
                    ))
                    .col(boolean(ImportRefs::DefaultBranch))
                    .col(date_time(ImportRefs::CreatedAt))
                    .col(date_time(ImportRefs::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_ref_path_name")
                    .unique()
                    .table(ImportRefs::Table)
                    .col(ImportRefs::RepoId)
                    .col(ImportRefs::RefName)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_refs_repo_id")
                    .table(ImportRefs::Table)
                    .col(ImportRefs::RepoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitRepo::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitRepo::Id))
                    .col(text(GitRepo::RepoPath))
                    .col(text(GitRepo::RepoName))
                    .col(date_time(GitRepo::CreatedAt))
                    .col(date_time(GitRepo::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_ir_path")
                    .unique()
                    .table(GitRepo::Table)
                    .col(GitRepo::RepoPath)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ir_repo_path")
                    .table(GitRepo::Table)
                    .col(GitRepo::RepoPath)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitCommit::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitCommit::Id))
                    .col(big_integer(GitCommit::RepoId))
                    .col(string(GitCommit::CommitId))
                    .col(string(GitCommit::Tree))
                    .col(json(GitCommit::ParentsId))
                    .col(text_null(GitCommit::Author))
                    .col(text_null(GitCommit::Committer))
                    .col(text_null(GitCommit::Content))
                    .col(date_time(GitCommit::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_c_git_repo_id")
                    .unique()
                    .table(GitCommit::Table)
                    .col(GitCommit::RepoId)
                    .col(GitCommit::CommitId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ic_git_id")
                    .table(GitCommit::Table)
                    .col(GitCommit::CommitId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ic_repo_id")
                    .table(GitCommit::Table)
                    .col(GitCommit::RepoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitTree::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitTree::Id))
                    .col(big_integer(GitTree::RepoId))
                    .col(string(GitTree::TreeId))
                    .col(binary(GitTree::SubTrees))
                    .col(integer(GitTree::Size))
                    .col(string(GitTree::CommitId))
                    .col(date_time(GitTree::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_t_git_repo")
                    .unique()
                    .table(GitTree::Table)
                    .col(GitTree::RepoId)
                    .col(GitTree::TreeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_t_git_id")
                    .table(GitTree::Table)
                    .col(GitTree::TreeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_t_repo_id")
                    .table(GitTree::Table)
                    .col(GitTree::RepoId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitBlob::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitBlob::Id))
                    .col(big_integer(GitBlob::RepoId))
                    .col(string(GitBlob::BlobId))
                    .col(string_null(GitBlob::Name))
                    .col(integer(GitBlob::Size))
                    .col(string(GitBlob::CommitId))
                    .col(date_time(GitBlob::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_b_git_repo")
                    .unique()
                    .table(GitBlob::Table)
                    .col(GitBlob::RepoId)
                    .col(GitBlob::BlobId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_b_git_id")
                    .table(GitBlob::Table)
                    .col(GitBlob::BlobId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitTag::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitTag::Id))
                    .col(big_integer(GitTag::RepoId))
                    .col(string(GitTag::TagId))
                    .col(string(GitTag::ObjectId))
                    .col(string(GitTag::ObjectType))
                    .col(text(GitTag::TagName))
                    .col(text(GitTag::Tagger))
                    .col(text(GitTag::Message))
                    .col(date_time(GitTag::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_gtag_tag_id")
                    .unique()
                    .table(GitTag::Table)
                    .col(GitTag::TagId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RawBlob::Table)
                    .if_not_exists()
                    .col(pk_bigint(RawBlob::Id))
                    .col(string(RawBlob::Sha1))
                    .col(text_null(RawBlob::Content))
                    .col(string_null(RawBlob::FileType))
                    .col(enumeration(
                        RawBlob::StorageType,
                        Alias::new("storage_type_enum"),
                        StorageType::iter(),
                    ))
                    .col(binary_null(RawBlob::Data))
                    .col(text_null(RawBlob::LocalPath))
                    .col(text_null(RawBlob::RemoteUrl))
                    .col(date_time(RawBlob::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_rb_sha1")
                    .unique()
                    .table(RawBlob::Table)
                    .col(RawBlob::Sha1)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_rb_sha1")
                    .table(RawBlob::Table)
                    .col(RawBlob::Sha1)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitPr::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitPr::Id))
                    .col(big_integer(GitPr::Number))
                    .col(string(GitPr::Title))
                    .col(string(GitPr::State))
                    .col(date_time(GitPr::CreatedAt))
                    .col(date_time(GitPr::UpdatedAt))
                    .col(date_time_null(GitPr::ClosedAt))
                    .col(date_time_null(GitPr::MergedAt))
                    .col(string_null(GitPr::MergeCommitSha))
                    .col(big_integer(GitPr::RepoId))
                    .col(string(GitPr::SenderName))
                    .col(big_integer(GitPr::SenderId))
                    .col(string(GitPr::UserName))
                    .col(big_integer(GitPr::UserId))
                    .col(string(GitPr::CommitsUrl))
                    .col(string(GitPr::PatchUrl))
                    .col(string(GitPr::HeadLabel))
                    .col(string(GitPr::HeadRef))
                    .col(string(GitPr::BaseLabel))
                    .col(string(GitPr::BaseRef))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GitIssue::Table)
                    .if_not_exists()
                    .col(pk_bigint(GitIssue::Id))
                    .col(big_integer(GitIssue::Number))
                    .col(string(GitIssue::Title))
                    .col(string(GitIssue::SenderName))
                    .col(big_integer(GitIssue::SenderId))
                    .col(string(GitIssue::State))
                    .col(date_time(GitIssue::CreatedAt))
                    .col(date_time(GitIssue::UpdatedAt))
                    .col(date_time_null(GitIssue::ClosedAt))
                    .col(big_integer(GitIssue::RepoId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LfsLocks::Table)
                    .if_not_exists()
                    .col(string(LfsLocks::Id).primary_key())
                    .col(text(LfsLocks::Data))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LfsObjects::Table)
                    .if_not_exists()
                    .col(string(LfsObjects::Oid).primary_key())
                    .col(big_integer(LfsObjects::Size))
                    .col(boolean(LfsObjects::Exist))
                    .col(boolean(LfsObjects::Splited))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LfsSplitRelations::Table)
                    .if_not_exists()
                    .col(string(LfsSplitRelations::OriOid))
                    .col(string(LfsSplitRelations::SubOid))
                    .col(big_integer(LfsSplitRelations::Offset))
                    .col(big_integer(LfsSplitRelations::Size))
                    .primary_key(
                        Index::create()
                            .col(LfsSplitRelations::OriOid)
                            .col(LfsSplitRelations::SubOid)
                            .col(LfsSplitRelations::Offset),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmNode::Table)
                    .if_not_exists()
                    .col(string(ZtmNode::PeerId).primary_key())
                    .col(string(ZtmNode::Hub))
                    .col(string(ZtmNode::AgentName))
                    .col(string(ZtmNode::ServiceName))
                    .col(string(ZtmNode::Type))
                    .col(boolean(ZtmNode::Online))
                    .col(big_integer(ZtmNode::LastOnlineTime))
                    .col(integer(ZtmNode::ServicePort))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmRepoInfo::Table)
                    .if_not_exists()
                    .col(string(ZtmRepoInfo::Identifier).primary_key())
                    .col(string(ZtmRepoInfo::Name))
                    .col(string(ZtmRepoInfo::Origin))
                    .col(big_integer(ZtmRepoInfo::UpdateTime))
                    .col(string(ZtmRepoInfo::Commit))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmLfsInfo::Table)
                    .if_not_exists()
                    .col(pk_bigint(ZtmLfsInfo::Id))
                    .col(string(ZtmLfsInfo::FileHash))
                    .col(string(ZtmLfsInfo::HashType))
                    .col(big_integer(ZtmLfsInfo::FileSize))
                    .col(big_integer(ZtmLfsInfo::CreationTime))
                    .col(string(ZtmLfsInfo::PeerId))
                    .col(string(ZtmLfsInfo::Origin))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmNostrEvent::Table)
                    .if_not_exists()
                    .col(string(ZtmNostrEvent::Id).primary_key())
                    .col(string(ZtmNostrEvent::Pubkey))
                    .col(big_integer(ZtmNostrEvent::CreatedAt))
                    .col(integer(ZtmNostrEvent::Kind))
                    .col(text(ZtmNostrEvent::Tags))
                    .col(text(ZtmNostrEvent::Content))
                    .col(string(ZtmNostrEvent::Sig))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmNostrReq::Table)
                    .if_not_exists()
                    .col(string(ZtmNostrReq::Id).primary_key())
                    .col(string(ZtmNostrReq::SubscriptionId))
                    .col(text(ZtmNostrReq::Filters))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MqStorage::Table)
                    .if_not_exists()
                    .col(pk_bigint(MqStorage::Id))
                    .col(string_null(MqStorage::Category))
                    .col(timestamp(MqStorage::CreateTime))
                    .col(text_null(MqStorage::Content))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ZtmPathMapping::Table)
                    .if_not_exists()
                    .col(pk_bigint(ZtmPathMapping::Id))
                    .col(text(ZtmPathMapping::Alias))
                    .col(text(ZtmPathMapping::RepoPath))
                    .col(date_time(ZtmPathMapping::CreatedAt))
                    .col(date_time(ZtmPathMapping::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_alias")
                    .unique()
                    .table(ZtmPathMapping::Table)
                    .col(ZtmPathMapping::Alias)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(pk_bigint(User::Id))
                    .col(text(User::Name))
                    .col(text(User::Email))
                    .col(text(User::AvatarUrl))
                    .col(boolean(User::IsGithub))
                    .col(date_time(User::CreatedAt))
                    .col(date_time_null(User::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_email")
                    .unique()
                    .table(User::Table)
                    .col(User::Email)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SshKeys::Table)
                    .if_not_exists()
                    .col(pk_bigint(SshKeys::Id))
                    .col(big_integer(SshKeys::UserId))
                    .col(text(SshKeys::Title))
                    .col(text(SshKeys::SshKey))
                    .col(text(SshKeys::Finger))
                    .col(date_time(SshKeys::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_user_id")
                    .table(SshKeys::Table)
                    .col(SshKeys::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ssh_key_finger")
                    .table(SshKeys::Table)
                    .col(SshKeys::Finger)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(AccessToken::Table)
                    .if_not_exists()
                    .col(pk_bigint(AccessToken::Id))
                    .col(big_integer(AccessToken::UserId))
                    .col(text(AccessToken::Token))
                    .col(date_time(AccessToken::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_token_user_id")
                    .table(AccessToken::Table)
                    .col(AccessToken::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_token")
                    .table(AccessToken::Table)
                    .col(AccessToken::Token)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Builds::Table)
                    .if_not_exists()
                    .col(uuid(Builds::BuildId).primary_key())
                    .col(string(Builds::Output))
                    .col(integer_null(Builds::ExitCode))
                    .col(timestamp(Builds::StartAt))
                    .col(timestamp(Builds::EndAt))
                    .col(string(Builds::RepoName))
                    .col(string(Builds::Target))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum MegaCommit {
    Table,
    Id,
    CommitId,
    Tree,
    ParentsId,
    Author,
    Committer,
    Content,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MegaTree {
    Table,
    Id,
    TreeId,
    SubTrees,
    Size,
    CommitId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MegaBlob {
    Table,
    Id,
    BlobId,
    CommitId,
    Name,
    Size,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MegaTag {
    Table,
    Id,
    TagId,
    ObjectId,
    ObjectType,
    TagName,
    Tagger,
    Message,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MegaMr {
    Table,
    Id,
    Link,
    Title,
    MergeDate,
    Status,
    Path,
    FromHash,
    ToHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MegaConversation {
    Table,
    Id,
    Link,
    UserId,
    ConvType,
    Comment,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MegaIssue {
    Table,
    Id,
    Link,
    Title,
    Owner,
    Status,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
}

#[derive(DeriveIden)]
enum MegaRefs {
    Table,
    Id,
    Path,
    RefName,
    RefCommitHash,
    RefTreeHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ImportRefs {
    Table,
    Id,
    RepoId,
    RefName,
    RefGitId,
    RefType,
    DefaultBranch,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GitRepo {
    Table,
    Id,
    RepoPath,
    RepoName,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum GitCommit {
    Table,
    Id,
    RepoId,
    CommitId,
    Tree,
    ParentsId,
    Author,
    Committer,
    Content,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GitTree {
    Table,
    Id,
    RepoId,
    TreeId,
    SubTrees,
    Size,
    CommitId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GitBlob {
    Table,
    Id,
    RepoId,
    BlobId,
    Name,
    Size,
    CommitId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GitTag {
    Table,
    Id,
    RepoId,
    TagId,
    ObjectId,
    ObjectType,
    TagName,
    Tagger,
    Message,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RawBlob {
    Table,
    Id,
    Sha1,
    Content,
    FileType,
    StorageType,
    Data,
    LocalPath,
    RemoteUrl,
    CreatedAt,
}

#[derive(DeriveIden)]
enum GitPr {
    Table,
    Id,
    Number,
    Title,
    State,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    MergedAt,
    MergeCommitSha,
    RepoId,
    SenderName,
    SenderId,
    UserName,
    UserId,
    CommitsUrl,
    PatchUrl,
    HeadLabel,
    HeadRef,
    BaseLabel,
    BaseRef,
}

#[derive(DeriveIden)]
enum GitIssue {
    Table,
    Id,
    Number,
    Title,
    SenderName,
    SenderId,
    State,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    RepoId,
}

#[derive(DeriveIden)]
enum LfsLocks {
    Table,
    Id,
    Data,
}

#[derive(DeriveIden)]
enum LfsObjects {
    Table,
    Oid,
    Size,
    Exist,
    Splited,
}

#[derive(DeriveIden)]
enum LfsSplitRelations {
    Table,
    OriOid,
    SubOid,
    Offset,
    Size,
}

#[derive(DeriveIden)]
enum ZtmNode {
    Table,
    PeerId,
    Hub,
    AgentName,
    ServiceName,
    Type,
    Online,
    LastOnlineTime,
    ServicePort,
}

#[derive(DeriveIden)]
enum ZtmRepoInfo {
    Table,
    Identifier,
    Name,
    Origin,
    UpdateTime,
    Commit,
}

#[derive(DeriveIden)]
enum ZtmLfsInfo {
    Table,
    Id,
    FileHash,
    HashType,
    FileSize,
    CreationTime,
    PeerId,
    Origin,
}

#[derive(DeriveIden)]
enum ZtmNostrEvent {
    Table,
    Id,
    Pubkey,
    CreatedAt,
    Kind,
    Tags,
    Content,
    Sig,
}

#[derive(DeriveIden)]
enum ZtmNostrReq {
    Table,
    Id,
    SubscriptionId,
    Filters,
}

#[derive(DeriveIden)]
enum MqStorage {
    Table,
    Id,
    Category,
    CreateTime,
    Content,
}

#[derive(DeriveIden)]
enum ZtmPathMapping {
    Table,
    Id,
    Alias,
    RepoPath,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Name,
    Email,
    AvatarUrl,
    IsGithub,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SshKeys {
    Table,
    Id,
    UserId,
    Title,
    SshKey,
    Finger,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AccessToken {
    Table,
    Id,
    UserId,
    Token,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Builds {
    Table,
    BuildId,
    Output,
    ExitCode,
    StartAt,
    EndAt,
    RepoName,
    Target,
}

#[derive(DeriveIden)]
struct StorageTypeEnum;

#[derive(Iden, EnumIter)]
pub enum StorageType {
    Database,
    LocalFs,
    AwsS3,
}

#[derive(DeriveIden)]
struct MergeStatusEnum;

#[derive(Iden, EnumIter)]
pub enum MergeStatus {
    Open,
    Merged,
    Closed,
}

#[derive(DeriveIden)]
struct RefTypeEnum;
#[derive(Iden, EnumIter)]
pub enum RefType {
    Branch,
    Tag,
}

#[derive(DeriveIden)]
struct ConvTypeEnum;
#[derive(Iden, EnumIter)]
pub enum ConvType {
    Comment,
    Deploy,
    Commit,
    ForcePush,
    Edit,
    Review,
    Approve,
    MergeQueue,
    Merged,
    Closed,
    Reopen,
}
