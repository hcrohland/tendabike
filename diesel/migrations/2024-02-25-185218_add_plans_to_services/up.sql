alter table services add column plans uuid[] DEFAULT ARRAY[]::uuid[];
