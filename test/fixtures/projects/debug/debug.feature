Feature: debug

  Background:
    * def base = 'from background'

  Scenario: variables
    * def name = 'grace'
    * def user = { id: 7 }
    * def called = call read('called.feature') { arg: 'in' }
    * match user.id == 7

  Scenario: second
    * def other = 'second'
