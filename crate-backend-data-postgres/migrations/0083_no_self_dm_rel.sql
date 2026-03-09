alter table dm add constraint no_self_dm check (user_a_id != user_b_id);
alter table user_relationship add constraint no_self_relation check (user_id != other_id);
