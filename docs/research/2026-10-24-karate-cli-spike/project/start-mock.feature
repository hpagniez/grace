@ignore
Feature: start the mock once per suite

Scenario:
  * def server = karate.start('mocks/api-mock.feature')
  * def port = server.port
