Feature: orders in an in-memory H2 database, through a compiled helper

  Background:
    * def db = Java.type('com.example.qa.DbHelper')
    * db.update('create table if not exists orders (id int primary key, item varchar(40))', [])
    * db.update('delete from orders', [])
    * db.update('insert into orders values (?, ?)', [1, 'book'])

  Scenario: read with a parameter
    * def rows = db.rows('select item from orders where id = ?', [1])
    * match rows == [{ item: 'book' }]

  Scenario: a failed transaction leaves nothing behind
    * def committed = db.inOneTransaction(["insert into orders values (2, 'pen')", "insert into orders values (1, 'twice')"])
    * match committed == false
    * match db.rows('select count(*) as n from orders', []) == [{ n: 1 }]
