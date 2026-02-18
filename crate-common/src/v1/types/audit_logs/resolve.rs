use std::collections::HashSet;

use crate::v1::types::{
    ApplicationId, AuditLogEntry, AuditLogEntryType, ChannelId, PermissionOverwriteType, UserId,
    WebhookId,
};

/// the set of extra data that should be resolved
#[derive(Debug, Default)]
pub struct AuditLogResolve {
    /// fetch thread channels for these ids; don't include active threads
    pub threads: HashSet<ChannelId>,

    /// fetch users and room_members for these ids
    pub users: HashSet<UserId>,

    pub applications: HashSet<ApplicationId>,

    /// fetch webhooks for these ids
    pub webhooks: HashSet<WebhookId>,
}

impl AuditLogResolve {
    /// add everything that needs to be resolved from this entry
    pub fn add(&mut self, entry: &AuditLogEntry) {
        self.users.insert(entry.user_id);

        if let Some(app_id) = entry.application_id {
            self.applications.insert(app_id);
        }

        match &entry.ty {
            AuditLogEntryType::ChannelCreate {
                channel_id,
                channel_type,
                ..
            } if channel_type.is_thread() => {
                self.threads.insert(*channel_id);
            }
            AuditLogEntryType::ChannelUpdate {
                channel_id,
                channel_type,
                ..
            } if channel_type.is_thread() => {
                self.threads.insert(*channel_id);
            }
            AuditLogEntryType::MemberKick { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::MemberBan { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::MemberUnban { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::MemberUpdate { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::RoleApply { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::RoleUnapply { user_id, .. } => {
                self.users.insert(*user_id);
            }
            AuditLogEntryType::BotAdd { bot_id } => {
                self.users.insert(*bot_id);
            }
            AuditLogEntryType::ThreadMemberAdd {
                thread_id, user_id, ..
            } => {
                self.threads.insert(*thread_id);
                self.users.insert(*user_id);
            }
            AuditLogEntryType::ThreadMemberRemove {
                thread_id, user_id, ..
            } => {
                self.threads.insert(*thread_id);
                self.users.insert(*user_id);
            }
            AuditLogEntryType::WebhookCreate { webhook_id, .. } => {
                self.webhooks.insert(*webhook_id);
            }
            AuditLogEntryType::WebhookUpdate { webhook_id, .. } => {
                self.webhooks.insert(*webhook_id);
            }
            AuditLogEntryType::WebhookDelete { webhook_id, .. } => {
                self.webhooks.insert(*webhook_id);
            }
            AuditLogEntryType::MessageDelete { channel_id, .. }
            | AuditLogEntryType::MessageVersionDelete { channel_id, .. }
            | AuditLogEntryType::MessageDeleteBulk { channel_id, .. }
            | AuditLogEntryType::MessageRemove { channel_id, .. }
            | AuditLogEntryType::MessageRestore { channel_id, .. }
            | AuditLogEntryType::ReactionDeleteAll { channel_id, .. }
            | AuditLogEntryType::ReactionDeleteKey { channel_id, .. }
            | AuditLogEntryType::ReactionDeleteUser { channel_id, .. }
            | AuditLogEntryType::MemberDisconnect { channel_id, .. }
            | AuditLogEntryType::MemberDisconnectAll { channel_id }
            | AuditLogEntryType::MessagePin { channel_id, .. }
            | AuditLogEntryType::MessageUnpin { channel_id, .. }
            | AuditLogEntryType::MessagePinReorder { channel_id }
            | AuditLogEntryType::RatelimitUpdate { channel_id, .. }
            | AuditLogEntryType::RatelimitDelete { channel_id, .. }
            | AuditLogEntryType::RatelimitDeleteAll { channel_id } => {
                self.threads.insert(*channel_id);
            }
            AuditLogEntryType::PermissionOverwriteCreate {
                channel_id,
                overwrite_id,
                ty,
                ..
            }
            | AuditLogEntryType::PermissionOverwriteUpdate {
                channel_id,
                overwrite_id,
                ty,
                ..
            }
            | AuditLogEntryType::PermissionOverwriteDelete {
                channel_id,
                overwrite_id,
                ty,
                ..
            } => {
                self.threads.insert(*channel_id);
                match ty {
                    PermissionOverwriteType::Role => {}
                    PermissionOverwriteType::User => {
                        self.users.insert((*overwrite_id).into());
                    }
                };
            }
            _ => {}
        }
    }
}
