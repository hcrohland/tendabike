ALTER TABLE "part_types" ADD COLUMN "order" integer NOT NULL DEFAULT '9999';

INSERT INTO part_types VALUES
(303,'Ski',303,'{}',9999),
(301,'Shoe',301,'{}',9999),
(304,'Whatever',304,'{}',9999),
(302,'Snowboard',302,'{}',9999),
(305,'SUP board',305,'{}',9999),
(306,'Windsurf Board',306,'{}',9999),
(307,'Kite Board',307,'{}',9999),
(308,'Rowing boat',308,'{}',9999),
(309,'binding',302,'{302}',9999),
(12,'derailleur',1,'{1}',12),
(16,'fork',1,'{1}',14),
(10,'seat post',1,'{1}',16),
(14,'chainring',1,'{13}',9),
(4,'chain',1,'{1}',11),
(13,'crank',1,'{1}',13),
(17,'rear shock',1,'{1}',15),
(11,'saddle',1,'{10}',17),
(9,'cassette',1,'{5}',10),
(1,'Bike',1,'{}',1),
(15,'brake rotor',1,'{2,5}',8),
(5,'rear wheel',1,'{1}',6),
(6,'brake pad',1,'{7,8}',4),
(7,'front brake',1,'{1}',2),
(3,'tire',1,'{2,5}',7),
(2,'front wheel',1,'{1}',5),
(8,'rear brake',1,'{1}',3),
(18,'pedal',1,'{13}',10);


INSERT INTO activity_types (id, name, gear) VALUES
(1,'Bike Ride',1),
(2,'Snowboard',302),
(3,'Running',301),
(4,'Hiking',301),
(5,'Virtual Ride',1),
(6,'Skiing',303),
(8,'Walk',301),
(7,'Splitboard Tour',302),
(10,'Skitour',303),
(0,'Whatever',304),
(9,'EBike Ride',1);
