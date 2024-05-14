ALTER TABLE "activities"
    RENAME COLUMN "energy" TO "power";
ALTER TABLE "usages"
    RENAME COLUMN "energy" TO "power";
update activities
set "power" = null;
update usages
set "power" = 0;