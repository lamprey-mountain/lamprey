use common::v1::types::{AuditLogEntryType, Channel, util::Changes};

pub fn calculate_audit_log_entry_from_create(channel: &Channel) -> AuditLogEntryType {
    let mut changes = Changes::new()
        .add("name", &channel.name)
        .add("description", &channel.description)
        .add("nsfw", &channel.nsfw)
        .add("user_limit", &channel.user_limit)
        .add("bitrate", &channel.bitrate)
        .add("type", &channel.ty)
        .add("parent_id", &channel.parent_id)
        .add("url", &channel.url)
        .add("invitable", &channel.invitable)
        .add("auto_archive_duration", &channel.auto_archive_duration)
        .add(
            "default_auto_archive_duration",
            &channel.default_auto_archive_duration,
        )
        .add("slowmode_thread", &channel.slowmode_thread)
        .add("slowmode_message", &channel.slowmode_message)
        .add(
            "default_slowmode_message",
            &channel.default_slowmode_message,
        )
        .add("tags", &channel.tags)
        .add("icon", &channel.icon)
        .add("locked", &channel.locked);

    if let Some(doc) = channel.document.as_ref() {
        changes = changes
            .add("document_draft", &doc.draft)
            .add("document_template", &doc.template)
            .add("document_slug", &doc.slug)
            .add("document_archived", &doc.archived.is_some())
            .add(
                "document_archived_reason",
                &doc.archived.as_ref().map(|a| &a.reason),
            )
            .add("document_published", &doc.published.is_some())
            .add(
                "document_published_revision",
                &doc.published.as_ref().map(|p| &p.revision),
            )
            .add(
                "document_published_unlisted",
                &doc.published.as_ref().map(|p| p.unlisted),
            );
    }

    if let Some(wiki) = channel.wiki.as_ref() {
        changes = changes
            .add("wiki_allow_indexing", &wiki.allow_indexing)
            .add("wiki_page_index", &wiki.page_index)
            .add("wiki_page_notfound", &wiki.page_notfound);
    }

    if let Some(cal) = channel.calendar.as_ref() {
        changes = changes
            .add("calendar_color", &cal.color)
            .add("calendar_default_timezone", &cal.default_timezone);
    }

    AuditLogEntryType::ChannelCreate {
        channel_id: channel.id,
        channel_type: channel.ty,
        changes: changes.build(),
    }
}

pub fn calculate_audit_log_entry_from_update(old: &Channel, new: &Channel) -> AuditLogEntryType {
    let mut changes = Changes::new()
        .change("name", &old.name, &new.name)
        .change("description", &old.description, &new.description)
        .change("nsfw", &old.nsfw, &new.nsfw)
        .change("user_limit", &old.user_limit, &new.user_limit)
        .change("bitrate", &old.bitrate, &new.bitrate)
        .change("type", &old.ty, &new.ty)
        .change("parent_id", &old.parent_id, &new.parent_id)
        .change("url", &old.url, &new.url)
        .change("invitable", &old.invitable, &new.invitable)
        .change(
            "auto_archive_duration",
            &old.auto_archive_duration,
            &new.auto_archive_duration,
        )
        .change(
            "default_auto_archive_duration",
            &old.default_auto_archive_duration,
            &new.default_auto_archive_duration,
        )
        .change(
            "slowmode_thread",
            &old.slowmode_thread,
            &new.slowmode_thread,
        )
        .change(
            "slowmode_message",
            &old.slowmode_message,
            &new.slowmode_message,
        )
        .change(
            "default_slowmode_message",
            &old.default_slowmode_message,
            &new.default_slowmode_message,
        )
        .change("tags", &old.tags, &new.tags)
        .change("icon", &old.icon, &new.icon)
        .change("locked", &old.locked, &new.locked)
        .change("archived", &old.is_archived(), &new.is_archived());

    if let (Some(old_doc), Some(new_doc)) = (old.document.as_ref(), new.document.as_ref()) {
        changes = changes
            .change("document_draft", &old_doc.draft, &new_doc.draft)
            .change("document_template", &old_doc.template, &new_doc.template)
            .change("document_slug", &old_doc.slug, &new_doc.slug)
            .change(
                "document_archived",
                &old_doc.archived.is_some(),
                &new_doc.archived.is_some(),
            )
            .change(
                "document_archived_reason",
                &old_doc.archived.as_ref().map(|a| &a.reason),
                &new_doc.archived.as_ref().map(|a| &a.reason),
            )
            .change(
                "document_published",
                &old_doc.published.is_some(),
                &new_doc.published.is_some(),
            )
            .change(
                "document_published_revision",
                &old_doc.published.as_ref().map(|p| &p.revision),
                &new_doc.published.as_ref().map(|p| &p.revision),
            )
            .change(
                "document_published_unlisted",
                &old_doc.published.as_ref().map(|p| p.unlisted),
                &new_doc.published.as_ref().map(|p| p.unlisted),
            );
    }

    if let (Some(old_wiki), Some(new_wiki)) = (old.wiki.as_ref(), new.wiki.as_ref()) {
        changes = changes
            .change(
                "wiki_allow_indexing",
                &old_wiki.allow_indexing,
                &new_wiki.allow_indexing,
            )
            .change(
                "wiki_page_index",
                &old_wiki.page_index,
                &new_wiki.page_index,
            )
            .change(
                "wiki_page_notfound",
                &old_wiki.page_notfound,
                &new_wiki.page_notfound,
            );
    }

    if let (Some(old_cal), Some(new_cal)) = (old.calendar.as_ref(), new.calendar.as_ref()) {
        changes = changes
            .change("calendar_color", &old_cal.color, &new_cal.color)
            .change(
                "calendar_default_timezone",
                &old_cal.default_timezone,
                &new_cal.default_timezone,
            );
    }

    AuditLogEntryType::ChannelUpdate {
        channel_id: new.id,
        channel_type: new.ty,
        changes: changes.build(),
    }
}
