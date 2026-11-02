package support;

import com.acme.db.Db;

/** A test helper the build compiles into target/test-classes; it calls the company library, which calls the driver. */
public final class Users {
    private Users() {}

    public static String addAndRead(String url, String name) throws Exception {
        Db.update(url, "create table if not exists users (name text)");
        Db.update(url, "delete from users");
        Db.update(url, "insert into users values ('" + name + "')");
        return Db.first(url, "select name from users");
    }
}
