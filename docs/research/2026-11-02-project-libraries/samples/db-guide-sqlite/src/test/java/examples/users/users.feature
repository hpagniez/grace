Feature: users in a local SQLite database, through DbUtils

  Background:
    * def DbUtils = Java.type('com.mycompany.DbUtils')
    # setup: a known table before each scenario
    * DbUtils.execute('create table if not exists users (id integer primary key, name text not null unique, email text)')
    * DbUtils.execute('delete from users')
    * DbUtils.execute("insert into users (id, name, email) values (1, 'ada', 'ada@example.com'), (2, 'grace', 'grace@example.com')")
    # cleanup: an empty table after each scenario
    * configure afterScenario = function(){ DbUtils.execute('delete from users') }

  Scenario: read every row
    * def users = DbUtils.query('select id, name from users order by id')
    * match users == [{ id: 1, name: 'ada' }, { id: 2, name: 'grace' }]

  Scenario: read with a parameter
    * def users = DbUtils.queryWithParams('select email from users where name = ?', 'grace')
    * match users == [{ email: 'grace@example.com' }]

  Scenario: change rows and count them
    * def changed = DbUtils.execute("update users set email = 'ada@lovelace.org' where name = 'ada'")
    * match changed == 1
    * match DbUtils.queryWithParams('select email from users where id = ?', 1)[0].email == 'ada@lovelace.org'

  Scenario: a transaction that commits
    * DbUtils.executeInTransaction("insert into users (id, name) values (3, 'alan')", "update users set email = 'alan@example.com' where id = 3")
    * match DbUtils.query('select count(*) as n from users')[0].n == 3

  Scenario: a transaction that fails leaves nothing behind
    # the second insert breaks the unique name: the first one is rolled back too
    * def failed = false
    * eval
      """
      try {
        DbUtils.executeInTransaction("insert into users (id, name) values (4, 'edsger')", "insert into users (id, name) values (5, 'ada')");
      } catch (e) {
        failed = true;
      }
      """
    * match failed == true
    * match DbUtils.queryWithParams('select count(*) as n from users where name = ?', 'edsger')[0].n == 0
