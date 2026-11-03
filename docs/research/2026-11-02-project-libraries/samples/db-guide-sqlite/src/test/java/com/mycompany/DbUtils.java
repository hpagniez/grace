package com.mycompany;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.ResultSetMetaData;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Database access for the features, at the path, name and methods of Karate's database testing guide, written for this
 * sample against a local SQLite file. Rows come back as lists of maps, which Karate reads as JSON.
 */
public final class DbUtils {
    // a file database under target/, relative to the working directory: the project folder in Grace, Maven and the IDE
    private static final String URL = "jdbc:sqlite:target/test.db";

    private DbUtils() {}

    private static Connection connect() throws SQLException {
        return DriverManager.getConnection(URL);
    }

    private static List<Map<String, Object>> rows(ResultSet r) throws SQLException {
        ResultSetMetaData m = r.getMetaData();
        List<Map<String, Object>> out = new ArrayList<>();
        while (r.next()) {
            Map<String, Object> row = new LinkedHashMap<>();
            for (int i = 1; i <= m.getColumnCount(); i++) row.put(m.getColumnLabel(i), r.getObject(i));
            out.add(row);
        }
        return out;
    }

    /** Runs a SELECT. */
    public static List<Map<String, Object>> query(String sql) throws SQLException {
        try (Connection c = connect(); Statement s = c.createStatement(); ResultSet r = s.executeQuery(sql)) {
            return rows(r);
        }
    }

    /** Runs a SELECT with `?` parameters, bound in order. */
    public static List<Map<String, Object>> queryWithParams(String sql, Object... params) throws SQLException {
        try (Connection c = connect(); PreparedStatement s = c.prepareStatement(sql)) {
            for (int i = 0; i < params.length; i++) s.setObject(i + 1, params[i]);
            try (ResultSet r = s.executeQuery()) {
                return rows(r);
            }
        }
    }

    /** Runs an INSERT, UPDATE, DELETE or DDL statement; returns the rows it changed. */
    public static int execute(String sql) throws SQLException {
        try (Connection c = connect(); Statement s = c.createStatement()) {
            return s.executeUpdate(sql);
        }
    }

    /** Runs the statements in one transaction; if one fails, rolls all back and rethrows. */
    public static void executeInTransaction(String... sqls) throws SQLException {
        try (Connection c = connect()) {
            c.setAutoCommit(false);
            try (Statement s = c.createStatement()) {
                for (String sql : sqls) s.executeUpdate(sql);
                c.commit();
            } catch (SQLException e) {
                c.rollback();
                throw e;
            }
        }
    }
}
