package com.acme.db;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

/** A stand-in for a company library: published only to the company's own repository, never to Maven Central. */
public final class Db {
    private Db() {}

    public static void update(String url, String sql) throws SQLException {
        try (Connection c = DriverManager.getConnection(url); Statement s = c.createStatement()) {
            s.executeUpdate(sql);
        }
    }

    /** The first column of the first row, or null. */
    public static String first(String url, String sql) throws SQLException {
        try (Connection c = DriverManager.getConnection(url);
                Statement s = c.createStatement();
                ResultSet r = s.executeQuery(sql)) {
            return r.next() ? r.getString(1) : null;
        }
    }
}
