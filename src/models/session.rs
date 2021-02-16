use crate::schema::sessions;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use ipnetwork::IpNetwork;
use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::net::IpAddr;

const TOKEN_LENGTH: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Identifiable, Queryable)]
#[table_name = "sessions"]
pub struct Session {
    pub id: i32,
    pub user_id: i32,
    hashed_token: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub revoked: bool,
    last_ip_address: IpNetwork,
    pub last_user_agent: String,
}

impl Session {
    /// Creates a new session builder that can be used to insert new session
    /// into the database.
    ///
    /// ```
    /// let session = Session::new()
    ///    .user_id(user.id)
    ///    .token(&token)
    ///    .last_ip_address(ip_addr)
    ///    .last_user_agent(user_agent)
    ///    .build()?
    ///    .insert(&conn)?;
    /// ```
    ///
    /// New tokens can be generated by using `Session::generate_token()`.
    pub fn new() -> NewSessionBuilder<'static> {
        NewSessionBuilder::default()
    }

    /// Looks for an unrevoked session with the given `token` in the database
    /// and returns `Some(Session)` if successful.
    ///
    /// If the session exists then the `last_used_at`, `last_ip_address` and
    /// `last_user_agent` fields will be updated in the same step.
    pub fn find_by_token_and_update(
        conn: &PgConnection,
        token: &str,
        ip_address: IpAddr,
        user_agent: &str,
    ) -> Result<Option<Self>, diesel::result::Error> {
        let hashed_token = Self::hash_token(token);

        let sessions = sessions::table
            .filter(sessions::revoked.eq(false))
            .filter(sessions::hashed_token.eq(hashed_token.as_slice()));

        // If the database is in read only mode, we can't update these fields.
        // Try updating in a new transaction, if that fails, fall back to reading
        conn.transaction(|| {
            diesel::update(sessions)
                .set((
                    sessions::last_used_at.eq(diesel::dsl::now),
                    sessions::last_ip_address.eq(IpNetwork::from(ip_address)),
                    sessions::last_user_agent.eq(user_agent),
                ))
                .get_result(conn)
                .optional()
        })
        .or_else(|_| sessions.first(conn).optional())
    }

    /// Looks for a unrevoked sessions with the given `user_id` in the database
    /// and returns a list if successful.
    pub fn find_by_user_id(
        conn: &PgConnection,
        user_id: i32,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        sessions::table
            .filter(sessions::revoked.eq(false))
            .filter(sessions::user_id.eq(user_id))
            .get_results(conn)
    }

    /// Looks for an unrevoked session with the given `id` and `user_id` and
    /// revokes it, if it exists. The `bool` return value indicates whether a
    /// corresponding session was found or not.
    pub fn revoke(
        conn: &PgConnection,
        user_id: i32,
        session_id: i32,
    ) -> Result<bool, diesel::result::Error> {
        let sessions = sessions::table
            .filter(sessions::id.eq(session_id))
            .filter(sessions::user_id.eq(user_id))
            .filter(sessions::revoked.eq(false));

        diesel::update(sessions)
            .set(sessions::revoked.eq(true))
            .execute(conn)
            .map(|changed_rows| changed_rows == 1)
    }

    /// Generates a new plaintext token
    ///
    /// Note that this needs to be hashed before saving it in the database!
    pub fn generate_token() -> String {
        crate::util::generate_secure_alphanumeric_string(TOKEN_LENGTH)
    }

    /// Calculates the SHA256 hash of the given `token` so that it can safely
    /// be stored in the database.
    fn hash_token(token: &str) -> GenericArray<u8, U32> {
        Sha256::digest(token.as_bytes())
    }

    /// Returns the `IpAddr` of the last time that this session was used.
    pub fn last_ip_address(&self) -> IpAddr {
        self.last_ip_address.ip()
    }
}

#[derive(Builder, Clone, Debug, PartialEq, Eq, Insertable)]
#[table_name = "sessions"]
pub struct NewSession<'a> {
    user_id: i32,
    #[builder(private)]
    hashed_token: Vec<u8>,
    #[builder(private, setter(name = "_last_ip_address"))]
    last_ip_address: IpNetwork,
    #[builder(setter(into))]
    last_user_agent: Cow<'a, str>,
}

impl NewSession<'_> {
    /// Inserts this new session record into the `sessions` database table and
    /// returns a corresponding `Session` struct if successful.
    pub fn insert(self, conn: &PgConnection) -> Result<Session, diesel::result::Error> {
        diesel::insert_into(sessions::table)
            .values(self)
            .get_result(conn)
    }
}

impl<'a> NewSessionBuilder<'a> {
    /// Calculates the hash of the given token and sets the `hashed_token`
    /// field accordingly.
    pub fn token(&mut self, token: &str) -> &mut Self {
        let hashed_token = Session::hash_token(token);
        self.hashed_token(hashed_token.to_vec())
    }

    /// Converts the `IpAddr` to an `IpNetwork` and updates the
    /// corresponding field.
    pub fn last_ip_address(&mut self, ip_address: IpAddr) -> &mut Self {
        self._last_ip_address(IpNetwork::from(ip_address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewUser, User};
    use crate::schema::users;
    use crate::test_util::pg_connection;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn test_session() {
        let conn = pg_connection();

        // insert a new user so that the foreign key works
        let new_user = NewUser::new(42, "johndoe", None, None, "secret123");

        let user: User = assert_ok!(diesel::insert_into(users::table)
            .values(new_user)
            .get_result(&conn));

        // insert a new session
        let ip_addr = "192.168.0.42";
        let user_agent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36";

        let token = Session::generate_token();
        let new_session = assert_ok!(Session::new()
            .user_id(user.id)
            .token(&token)
            .last_ip_address(IpAddr::from_str(ip_addr).unwrap())
            .last_user_agent(user_agent)
            .build());

        let session = assert_ok!(new_session.insert(&conn));
        assert_eq!(session.user_id, user.id);
        assert_eq!(session.hashed_token, Session::hash_token(&token).to_vec());
        assert_eq!(session.revoked, false);
        assert_eq!(
            session.last_ip_address(),
            IpAddr::from_str(ip_addr).unwrap()
        );
        assert_eq!(session.last_user_agent, user_agent);

        // query the session by `token`
        let ip_addr = "192.168.0.1";
        let user_agent =
            "Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0";

        let query_result = assert_ok!(Session::find_by_token_and_update(
            &conn,
            &token,
            IpAddr::from_str(ip_addr).unwrap(),
            user_agent
        ));
        let session = assert_some!(query_result);
        assert_eq!(session.user_id, user.id);
        assert_eq!(session.hashed_token, Session::hash_token(&token).to_vec());
        assert_eq!(session.revoked, false);
        assert_eq!(
            session.last_ip_address(),
            IpAddr::from_str(ip_addr).unwrap()
        );
        assert_eq!(session.last_user_agent, user_agent);

        // query the session by an unknown `token`
        let query_result = assert_ok!(Session::find_by_token_and_update(
            &conn,
            "some-other-token",
            IpAddr::from_str(ip_addr).unwrap(),
            user_agent
        ));
        assert_none!(query_result);

        // find all session by `user_id`
        let query_result = assert_ok!(Session::find_by_user_id(&conn, user.id));
        assert_eq!(query_result.len(), 1);

        // revoke the session
        assert_eq!(
            assert_ok!(Session::revoke(&conn, user.id, session.id)),
            true
        );

        // query the revoked session
        let query_result = assert_ok!(Session::find_by_token_and_update(
            &conn,
            &token,
            IpAddr::from_str(ip_addr).unwrap(),
            user_agent
        ));
        assert_none!(query_result);

        // try to revoke the session again
        assert_eq!(
            assert_ok!(Session::revoke(&conn, user.id, session.id)),
            false
        );

        // try to revoke a different session
        assert_eq!(
            assert_ok!(Session::revoke(&conn, user.id, session.id + 42)),
            false
        );

        // find all session by `user_id`
        let query_result = assert_ok!(Session::find_by_user_id(&conn, user.id));
        assert_eq!(query_result.len(), 0);
    }
}