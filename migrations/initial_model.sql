-- Database generated with pgModeler (PostgreSQL Database Modeler).
-- pgModeler version: 1.0.6
-- PostgreSQL version: 16.0
-- Project Site: pgmodeler.io
-- Model Author: ---
-- Tablespaces creation must be performed outside a multi lined SQL file. 
-- These commands were put in this file only as a convenience.
-- 
-- object: "Account" | type: TABLESPACE --
-- DROP TABLESPACE IF EXISTS "Account" CASCADE;
CREATE TABLESPACE "Account"
	OWNER postgres
	LOCATION 'hehe';

-- ddl-end --



-- Database creation must be performed outside a multi lined SQL file. 
-- These commands were put in this file only as a convenience.
-- 
-- object: rinhabackend | type: DATABASE --
-- DROP DATABASE IF EXISTS rinhabackend;
CREATE DATABASE rinhabackend
	ENCODING = 'UTF8';
-- ddl-end --


-- object: public.account | type: TABLE --
-- DROP TABLE IF EXISTS public.account CASCADE;
CREATE UNLOGGED TABLE public.account (
	id integer NOT NULL,
	CONSTRAINT "Account_pk" PRIMARY KEY (id)
);
-- ddl-end --
ALTER TABLE public.account OWNER TO postgres;
-- ddl-end --

-- object: public.transaction | type: TABLE --
-- DROP TABLE IF EXISTS public.transaction CASCADE;
CREATE UNLOGGED TABLE public.transaction (
	amount integer NOT NULL,
	description text NOT NULL,
	created_on timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
	account_id integer NOT NULL,
	CONSTRAINT transaction_pk PRIMARY KEY (account_id)
);
-- ddl-end --
COMMENT ON COLUMN public.transaction.amount IS E'Transaction amount';
-- ddl-end --
ALTER TABLE public.transaction OWNER TO postgres;
-- ddl-end --

-- object: account_fk | type: CONSTRAINT --
-- ALTER TABLE public.transaction DROP CONSTRAINT IF EXISTS account_fk CASCADE;
ALTER TABLE public.transaction ADD CONSTRAINT account_fk FOREIGN KEY (account_id)
REFERENCES public.account (id) MATCH FULL
ON DELETE CASCADE ON UPDATE CASCADE;
-- ddl-end --


