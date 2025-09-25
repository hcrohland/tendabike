<script lang="ts">
  import { Card, Heading, Hr, Li, List, P } from "flowbite-svelte";
</script>

<Card size="lg" class="p-4 sm:p-6 md:p-8 m-8">
  <div>
    <a
      href="http://www.strava.com"
      class="float-end items-start"
      target="_blank"
    >
      <img
        src="api_logo_pwrdBy_strava.png"
        alt="powered by Strava"
        height="20"
      />
    </a>
    <Heading tag="h3">Tend your bike for a happy ride</Heading>
    <P>Know how long your parts last and when to maintain them</P>
  </div>
  <Heading tag="h5">General use</Heading>
  <P>
    TendaBike fetches all your activities and bikes from Strava.
    <br /> You can create parts and attach them to your bikes. You can move
    parts between bikes.
    <br /> TendaBike will calculate the use for any part you create.
    <br /> You can also add service plans and record services and you will get reminded
    if a service plan is due
  </P>
  <P>
    When initially connecting TendaBike to Strava it will pull all bikes and
    activities into the database. Don't be afraid. You can correct anything
    later. Changes on Strava will be replicated into TendaBike
    <br /> After connecting you can start adding service plans, part and services.
    This is easiest done from the bike's detail page which you can find by clicking
    on the arrow right of the bike's name
  </P>

  <Heading tag="h5">Elevation data</Heading>
  <P>
    Strava only tracks elevation gain. This is unfortunate if you use lifts or
    shuttles.
    <br />Brakes are wearing off way faster downhill than uphill while e.g.
    chains wear on the uphill.
    <br /> If you are using a recent Garmin device and Garmin connect, we have you
    covered!
  </P>
  <P>This is what you can do:</P>
  <P>
    <List tag="ol">
      <Li>
        Stop recording in lifts and shuttles. By that Strava should not track
        the elevation gain for the shuttle
      </Li>
      <Li>
        Download the list of activities from the Garmin Connect website with
        German language settings.
        <br /> The resulting file will include the elevation loss
      </Li>
      <Li>
        Upload this file to TendaBike via 'Sync' &rightarrow; 'With CSV file'
      </Li>
    </List>
  </P>
  <Hr />
  <P>
    <div class="float-right">
      © Christoph Rohland 2019-2025 - Source Code at
      <a
        href="https://github.com/hcrohland/tendabike"
        title="Repository"
        class="inline"
      >
        GitHub
      </a>
    </div>
  </P>
</Card>
