Feature: mock without catch-all

Scenario: pathMatches('/hello')
  * def response = { message: 'hello' }
