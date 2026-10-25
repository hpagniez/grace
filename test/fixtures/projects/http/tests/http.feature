Feature: every kind of HTTP call

Background:
  * url baseUrl

Scenario: ok
  Given path 'hello'
  When method get
  Then status 200

Scenario: retry
  * configure retry = { count: 3, interval: 50 }
  Given path 'nothing'
  And retry until responseStatus == 200
  When method get

Scenario: server error
  Given path 'boom'
  When method get
  Then status 500

Scenario: refused
  * configure connectTimeout = 500
  Given url 'http://127.0.0.1:1/x'
  When method get
  Then status 200

Scenario: timeout
  * configure readTimeout = 500
  Given path 'slow'
  When method get
  Then status 200

Scenario: redirect followed
  Given path 'moved'
  When method get
  Then status 200

Scenario: redirect kept
  * configure followRedirects = false
  Given path 'moved'
  When method get
  Then status 302

Scenario: via helper
  * def result = call read('common/create-user.feature') { name: 'alice' }
  * match result.user.name == 'alice'

Scenario: xml
  Given path 'echo'
  And request <user><name>bob</name></user>
  When method post
  Then status 200

Scenario: secret header
  Given path 'hello'
  And header X-Token = karate.properties['api.token'] || 'none'
  When method get
  Then status 200
