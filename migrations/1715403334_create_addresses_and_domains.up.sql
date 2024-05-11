CREATE TABLE domains (
  id      SERIAL PRIMARY KEY,
  name    VARCHAR UNIQUE NOT NULL, -- noqa: RF04
  enabled BOOL DEFAULT TRUE
);

CREATE TABLE addresses (
  id          SERIAL PRIMARY KEY,
  address     VARCHAR NOT NULL,
  description VARCHAR,
  enabled     BOOLEAN DEFAULT TRUE,
  domain_id   INTEGER REFERENCES domains (id) ON DELETE CASCADE,
  user_id     INTEGER REFERENCES users (id) ON DELETE CASCADE,
  created_at  TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at  TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  UNIQUE (address, domain_id)
);

INSERT INTO domains (name) VALUES ('masked.rwx.im');
