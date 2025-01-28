with message_coalesced as (
    select *
    from (select *, row_number() over(partition by id order by version_id desc) as row_num
        from message)
    where row_num = 1
)
select count(*) from message_coalesced where thread_id = $1
