use crate::v1::types::media::Media as V1Media;
use crate::v1::types::message::{
    MessageDefaultMarkdown as V1MessageDefaultMarkdown, MessageType as V1MessageType,
};
use crate::v2::types::media::Media as V2Media;
use crate::v2::types::message::{
    MessageAttachment, MessageAttachmentType, MessageDefaultMarkdown as V2MessageDefaultMarkdown,
    MessageType as V2MessageType,
};

impl From<V1MessageType> for V2MessageType {
    fn from(v1: V1MessageType) -> Self {
        match v1 {
            V1MessageType::DefaultMarkdown(v1_md) => V2MessageType::DefaultMarkdown(v1_md.into()),
            V1MessageType::MessagePinned(p) => V2MessageType::MessagePinned(p),
            #[cfg(feature = "feat_message_move")]
            V1MessageType::MessagesMoved(m) => V2MessageType::MessagesMoved(m),
            V1MessageType::MemberAdd(m) => V2MessageType::MemberAdd(m),
            V1MessageType::MemberRemove(m) => V2MessageType::MemberRemove(m),
            V1MessageType::MemberJoin => V2MessageType::MemberJoin,
            V1MessageType::Call(c) => V2MessageType::Call(c),
            V1MessageType::ChannelRename(r) => V2MessageType::ChannelRename(r),
            V1MessageType::ChannelPingback(p) => V2MessageType::ChannelPingback(p),
            V1MessageType::ChannelMoved(m) => V2MessageType::ChannelMoved(m),
            V1MessageType::ChannelIcon(i) => V2MessageType::ChannelIcon(i),
            V1MessageType::ThreadCreated(t) => V2MessageType::ThreadCreated(t),
            V1MessageType::AutomodExecution(a) => V2MessageType::AutomodExecution(a),
        }
    }
}

impl From<V1MessageDefaultMarkdown> for V2MessageDefaultMarkdown {
    fn from(v1: V1MessageDefaultMarkdown) -> Self {
        V2MessageDefaultMarkdown {
            content: v1.content,
            attachments: v1.attachments.into_iter().map(|m| m.into()).collect(),
            reply_id: v1.reply_id,
            embeds: v1.embeds.into_iter().map(|e| e.into()).collect(),

            // TODO: use v1.metadata?
            // what happens if it doesnt pass validation?
            metadata: None,
        }
    }
}

impl From<V1Media> for MessageAttachment {
    fn from(v1: V1Media) -> Self {
        let v2_media: V2Media = v1.into();
        MessageAttachment {
            ty: MessageAttachmentType::Media { media: v2_media },
            spoiler: false,
        }
    }
}
