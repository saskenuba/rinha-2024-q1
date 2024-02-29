INSERT INTO account (id)
VALUES (1), (2), (3), (4), (5);

INSERT INTO transaction (amount, description, account_id)
VALUES (100000, 'Initial balance for account 1', 1)
     , (80000, 'Initial balance for account 2', 2)
     , (1000000, 'Initial balance for account 3', 3)
     , (10000000, 'Initial balance for account 4', 4)
     , (500000, 'Initial balance for account 5', 5)