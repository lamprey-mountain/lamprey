alter table email_queue add column html_body text;
alter table email_queue rename body to plain_text_body;
