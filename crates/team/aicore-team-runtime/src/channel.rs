use aicore_team_protocol::*;

use crate::TeamRuntimeError;

pub fn append_message_to_channel(
    channel: &mut TeamChannelState,
    messages: &mut Vec<TeamMessage>,
    mut message: TeamMessage,
) -> Result<TeamMessage, TeamRuntimeError> {
    if channel.status != TeamChannelStatus::Open {
        return Err(TeamRuntimeError::ChannelClosed);
    }
    channel.message_seq += 1;
    message.seq = channel.message_seq;
    messages.push(message.clone());
    Ok(message)
}
