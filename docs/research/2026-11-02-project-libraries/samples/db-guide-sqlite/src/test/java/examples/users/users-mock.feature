@ignore
Feature: the users service, a local mock that saves to the database

  Background:
    # Karate 2 turns Java off in mocks, whose data comes from requests; this local stand-in opts in
    * configure javaBridgeEnabled = true
    * def DbUtils = Java.type('com.mycompany.DbUtils')

  Scenario: pathMatches('/users') && methodIs('post')
    # a stand-in only: a real service binds its parameters instead of building SQL from the request
    * DbUtils.execute("insert into users (name, email) values ('" + request.name + "', '" + request.email + "')")
    * def responseStatus = 201
    * def response = { name: '#(request.name)' }
