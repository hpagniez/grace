package com.example.qa;

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
 * Database access for the features, in the shape of Karate's database testing guide: plain JDBC, the connection
 * settings as constants, loaded with Java.type. Lists come from JavaScript arrays.
 */
public final class DbHelper {
    // an in-memory H2 database that lives as long as the JVM
    private static final String URL = "jdbc:h2:mem:orders;DB_CLOSE_DELAY=-1";
    private static final String USER = "sa";
    private static final String PASSWORD = "";

    private DbHelper() {}

    private static Connection connect() throws SQLException {
        return DriverManager.getConnection(URL, USER, PASSWORD);
    }

    private static void bind(PreparedStatement s, List<Object> params) throws SQLException {
        for (int i = 0; i < params.size(); i++) s.setObject(i + 1, params.get(i));
    }

    /** Runs an update with `?` parameters; returns the rows it changed. */
    public static int update(String sql, List<Object> params) throws SQLException {
        try (Connection c = connect(); PreparedStatement s = c.prepareStatement(sql)) {
            bind(s, params);
            return s.executeUpdate();
        }
    }

    /** Runs a query with `?` parameters; each row is a map of lower-case column labels to values. */
    public static List<Map<String, Object>> rows(String sql, List<Object> params) throws SQLException {
        try (Connection c = connect(); PreparedStatement s = c.prepareStatement(sql)) {
            bind(s, params);
            try (ResultSet r = s.executeQuery()) {
                ResultSetMetaData m = r.getMetaData();
                List<Map<String, Object>> out = new ArrayList<>();
                while (r.next()) {
                    Map<String, Object> row = new LinkedHashMap<>();
                    for (int i = 1; i <= m.getColumnCount(); i++)
                        row.put(m.getColumnLabel(i).toLowerCase(), r.getObject(i));
                    out.add(row);
                }
                return out;
            }
        }
    }

    /** Runs every statement in one transaction: true if committed, false if one failed and all were rolled back. */
    public static boolean inOneTransaction(List<String> sqls) throws SQLException {
        try (Connection c = connect()) {
            c.setAutoCommit(false);
            try (Statement s = c.createStatement()) {
                for (String sql : sqls) s.executeUpdate(sql);
                c.commit();
                return true;
            } catch (SQLException e) {
                c.rollback();
                return false;
            }
        }
    }
}
