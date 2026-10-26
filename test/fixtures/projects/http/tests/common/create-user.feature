@ignore
Feature: helper that creates a user

Scenario:
  Given url baseUrl
  And path 'users'
  And request { name: '#(name)' }
  When method post
  Then status 201
  * def user = response
