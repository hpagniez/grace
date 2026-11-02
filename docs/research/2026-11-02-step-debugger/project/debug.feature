Feature: debug spike

  Background:
    * def base = 'from background'

  Scenario: variables of each kind
    * def name = 'grace'
    * def count = 41 + 1
    * def user = { id: 7, tags: ['a', 'b'] }
    * def list = [1, 2, 3]
    * xml doc = <root><item>x</item></root>
    * def twice = function(n){ return n * 2 }
    * def called = call read('called.feature') { arg: 'in' }
    * match twice(count) == 84

  Scenario: second scenario
    * def other = 'second'
    * print other
