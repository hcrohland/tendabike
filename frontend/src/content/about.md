# TendaBike

Know how long your parts last and when to maintain them.

## General use

TendaBike fetches all your activities and bikes from Strava. You can create parts and attach them to your bikes. You can move parts between bikes. TendaBike will calculate the use for any part you create. You can also add service plans and record services and you will get reminded if a service plan is due.

When initially connecting TendaBike to Strava it will pull all bikes and activities into the database. Don't be afraid. You can correct anything later. Changes on Strava will be replicated into TendaBike. After connecting you can start adding service plans, parts and services. This is easiest done from the bike's detail page.

## Elevation data

Strava only tracks elevation gain. This is unfortunate if you use lifts or shuttles. Brakes are wearing off way faster downhill than uphill while e.g. chains wear on the uphill. If you are using a recent Garmin device and Garmin Connect, we have you covered!

This is what you can do:

1. Stop recording in lifts and shuttles. By that Strava should not track the elevation gain for the shuttle
2. Download the list of activities from the Garmin Connect website with German language settings. The resulting file will include the elevation loss
3. Upload this file to TendaBike via Sync → CSV

---

© Christoph Rohland 2019-2026 - Source Code at [GitHub](https://github.com/hcrohland/tendabike)
