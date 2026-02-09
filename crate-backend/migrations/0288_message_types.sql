alter type message_type add value 'ChannelMove';
alter type message_type add value 'ChannelPingback';
alter type message_type add value 'Call';
alter type message_type rename value 'ThreadRename' to 'ChannelRename';
