Feature: a setup call shared by two scenarios

Background:
  * url baseUrl
  * def made = callonce read('common/create-user.feature') { name: 'shared' }

Scenario: first
  * match made.user.name == 'shared'

Scenario: second
  * match made.user.name == 'shared'
