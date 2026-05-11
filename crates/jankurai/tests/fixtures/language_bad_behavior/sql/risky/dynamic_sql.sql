DO $$
BEGIN
  EXECUTE 'SELECT * FROM users WHERE id = ' || user_id;
END $$;
