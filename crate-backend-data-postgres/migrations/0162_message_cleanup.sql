alter type message_type add value 'ThreadRename';
update message set type = 'DefaultMarkdown' where type = 'ThreadUpdate';
