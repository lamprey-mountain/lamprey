alter table session add column ip_addr inet;
alter table session add column user_agent text;
alter table session add column authorized_at timestamp;
alter table session add column deauthorized_at timestamp;
