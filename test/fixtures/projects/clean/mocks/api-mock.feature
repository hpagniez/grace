@ignore
Feature: deterministic API mock

Background:
  * def counter = { n: 0 }

Scenario: pathMatches('/hello') && methodIs('get')
  * def response = { message: 'hello', source: 'mock' }

Scenario: pathMatches('/users') && methodIs('post')
  * counter.n = counter.n + 1
  * def response = request
  * response.id = counter.n
  * def responseStatus = 201

Scenario:
  * def responseStatus = 404
  * def response = { error: 'not found' }
