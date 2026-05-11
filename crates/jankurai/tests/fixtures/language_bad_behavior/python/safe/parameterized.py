user_id = 7
cursor.execute("SELECT * FROM users WHERE id = %s", (user_id,))
