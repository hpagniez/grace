Feature: environment-dependent scenarios

Scenario: always
  * print 'karate.env =', karate.env
  * print 'timeout =', karate.properties['timeout']
  * print 'api.token =', karate.properties['api.token']

@env=qa
Scenario: qa only
  * print 'qa only ran'

@envnot=prod
Scenario: not prod
  * print 'not prod ran'

Scenario: slow
  * java.lang.Thread.sleep(3000)
