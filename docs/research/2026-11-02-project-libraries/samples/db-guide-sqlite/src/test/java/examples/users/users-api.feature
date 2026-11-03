Feature: check the database after an API call

  Background:
    * def DbUtils = Java.type('com.mycompany.DbUtils')
    * DbUtils.execute('create table if not exists users (id integer primary key, name text not null unique, email text)')
    * DbUtils.execute('delete from users')
    * configure afterScenario = function(){ DbUtils.execute('delete from users') }
    # a local stand-in for the service under test, which writes to the same database
    * def server = karate.start({ mock: 'users-mock.feature' })
    * url 'http://localhost:' + server.port

  Scenario: a created user is in the database
    Given path 'users'
    And request { name: 'barbara', email: 'barbara@example.com' }
    When method post
    Then status 201
    * def rows = DbUtils.queryWithParams('select name, email from users where name = ?', 'barbara')
    * match rows == [{ name: 'barbara', email: 'barbara@example.com' }]
