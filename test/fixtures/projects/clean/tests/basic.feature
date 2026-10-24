Feature: basic API checks

Background:
  * url baseUrl

@smoke
Scenario: passing hello
  Given path 'hello'
  When method get
  Then status 200
  And match response == { message: 'hello', source: 'mock' }
  * print 'env is', env, 'devFlag is', devFlag

@wip
Scenario: failing match
  Given path 'hello'
  When method get
  Then status 200
  # expected greeting is wrong on purpose
  And match response.message == 'goodbye'

Scenario: calls helper feature
  * def result = call read('common/create-user.feature') { name: 'alice' }
  * match result.user.name == 'alice'
  * match result.user.id == '#number'

@smoke @wip
Scenario: smoke but wip
  * def x = 1
  * match x == 1
