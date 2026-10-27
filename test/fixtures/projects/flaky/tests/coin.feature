Feature: coin

Scenario: coin
  * def coin = read('file:../coin.json')
  * match coin.heads == true

Scenario: steady
  * match 1 == 1
