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

Scenario: pathMatches('/slow')
  * def responseDelay = 2000
  * def response = { slow: true }

Scenario: pathMatches('/moved')
  * def responseStatus = 302
  * def responseHeaders = { Location: '/hello' }

Scenario: pathMatches('/boom')
  * def responseStatus = 500
  * def response = { error: 'boom' }

Scenario: pathMatches('/echo') && methodIs('post')
  * def response = request
  * def responseHeaders = { 'Content-Type': 'application/xml' }

Scenario:
  * def responseStatus = 404
  * def response = { error: 'not found' }
