use crate::prelude::*;

pub struct ParsedMentions {
    // ...
}

impl ParsedMentions {
    pub fn from_lamprey(_content: &str, _mentions: &lamprey::Mentions) -> Self {
        todo!()
    }

    pub fn from_discord(_content: &str) -> Self {
        todo!()
    }

    pub fn from_message(_message: &bridge::MessageData) -> Self {
        todo!()
    }

    pub fn render_lamprey(&self) -> String {
        todo!()
    }

    pub fn render_discord(&self) -> String {
        todo!()
    }

    pub fn allowed_lamprey(&self) -> lamprey::ParseMentions {
        todo!()
    }

    pub fn allowed_discord(&self) -> discord::CreateAllowedMentions {
        todo!()
    }
}

// TODO: test
// #[cfg(test)]
// mod tests;
