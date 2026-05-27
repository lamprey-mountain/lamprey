use crate::services::voice::ServiceVoice;
use crate::{Result, Error};
use common::v1::types::{
    voice::{
        messages::{SfuCommand, SignallingCommand},
        VoiceStateUpdate,
    },
    ChannelId, ConnectionId, Session,
};

// TODO: clean up stale voice states (delete voice state when connection is disconnected for too long)
// TODO: clean up voice states on clean disconnect (?)
// TODO: handle nonce (?) -- may not be necessary, might remove it later
// TODO: handle users' permissions being changed while connected to voice (propagate changes to sfus)

impl ServiceVoice {
    pub async fn handle_voice_connect(
        &self,
        session: Session,
        connection_id: ConnectionId,
        vs: VoiceStateUpdate,
        _nonce: Option<String>,
    ) -> Result<()> {
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        self.state_create(user_id, vs, Some(session.id), Some(connection_id))
            .await?;

        Ok(())
    }

    pub async fn handle_voice_dispatch(
        &self,
        session: Session,
        channel_id: ChannelId,
        _nonce: Option<String>,
        command: SignallingCommand,
    ) -> Result<()> {
        let user_id = session.user_id().ok_or(Error::UnauthSession)?;

        match command {
            SignallingCommand::Disconnect => {
                self.state_destroy(channel_id, user_id)?;
            }
            SignallingCommand::VoiceState { state } => {
                self.state_update(user_id, state).await?;
            }
            _ => {
                if let Some(sfu) = self.sfu_by_channel(channel_id) {
                    sfu.send(SfuCommand::Signalling {
                        user_id,
                        channel_id,
                        inner: command,
                    });
                }
            }
        }

        Ok(())
    }
}
