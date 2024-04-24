update parts
	SET last_used=res.max
	from (select gear, max(start) from activities group by gear) as res
	where parts.id = res.gear
	;     