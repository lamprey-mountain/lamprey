-- applications are bots, but app creation after 0270 did not set usr.bot
update usr set bot = true where id in (select id from application) and not bot;
