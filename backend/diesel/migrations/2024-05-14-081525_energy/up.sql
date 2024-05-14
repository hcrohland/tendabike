ALTER TABLE "activities"
    RENAME COLUMN "power" TO "energy";
ALTER TABLE "usages"
    RENAME COLUMN "power" TO "energy";
update activities
set energy = null;
update usages
set energy = 0;