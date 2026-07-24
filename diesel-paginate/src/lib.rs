use diesel::{
    pg::Pg,
    prelude::*,
    query_builder::*,
    query_dsl::methods::LoadQuery,
    sql_types::BigInt,
};

// TODO Add test for me
pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            per_page: DEFAULT_PER_PAGE,
            page,
            offset: (page - 1) * DEFAULT_PER_PAGE,
        }
    }
}

const DEFAULT_PER_PAGE: i64 = 10;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
    offset: i64,
}

impl<T> Paginated<T> {
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated {
            per_page,
            offset: (self.page - 1) * per_page,
            ..self
        }
    }

    pub fn load_and_count_pages<'a, U>(self, conn: &mut PgConnection) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<'a, PgConnection, (U, i64)>,
    {
        let per_page = self.per_page;
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.first().map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Ok((records, total_pages))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diesel::dsl::sql;

    fn base_query() -> impl QueryFragment<Pg> + QueryId {
        sql::<diesel::sql_types::Integer>("SELECT 1")
    }

    #[test]
    fn test_paginate_defaults() {
        let paged = base_query().paginate(1);
        assert_eq!(paged.page, 1);
        assert_eq!(paged.per_page, super::DEFAULT_PER_PAGE);
        assert_eq!(paged.offset, 0);
    }

    #[test]
    fn test_per_page_override() {
        let paged = base_query().paginate(2).per_page(15);

        assert_eq!(paged.per_page, 15);
        assert_eq!(paged.offset, 15); // (2-1) * 15 = 15
    }

    #[test]
    fn test_sql_generation() {
        let paged = base_query().paginate(3).per_page(20);
        let generated_sql = debug_query::<Pg, _>(&paged).to_string();

        assert!(generated_sql.contains("LIMIT $1"));
        assert!(generated_sql.contains("OFFSET $2"));
    }
}
