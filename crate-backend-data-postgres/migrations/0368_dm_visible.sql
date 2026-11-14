create table dm_visible (
    user_id uuid not null,
    channel_id uuid not null,
    foreign key (user_id) references usr(id) on delete cascade,
    foreign key (channel_id) references channel(id) on delete cascade,
    primary key (user_id, channel_id)
);

-- existing dms are visible for both users
insert into dm_visible (user_id, channel_id)
select user_a_id, channel_id from dm
union all
select user_b_id, channel_id from dm
on conflict do nothing;
