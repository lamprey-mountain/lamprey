create table message_attachment (
    version_id uuid,
    media_id uuid,
    ordering int not null,
    foreign key (version_id) references message (version_id),
    foreign key (media_id) references media (id),
    primary key (version_id, media_id)
);

-- somehow, there are messages that reference missing media
update message set attachments = (
    select array_agg(att.media_id order by ord)
    from unnest(attachments) with ordinality as att(media_id, ord)
    where exists (select 1 from media where media.id = att.media_id)
)
where exists (
    select 1 from unnest(attachments) as att(media_id) 
    where not exists (select 1 from media where id = att.media_id)
);

insert into message_attachment 
select message.version_id, att.media_id, att.ordering - 1
from message, unnest(message.attachments) with ordinality as att(media_id, ordering);

drop view att_json;
alter table message drop column attachments;
