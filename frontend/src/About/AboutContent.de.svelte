<script lang="ts">
  import { Heading, Hr, Li, List, P } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
</script>

<div class="p-4 sm:p-6 md:p-8 m-8 max-w-200">
  <a href="http://www.strava.com" class="float-end items-start" target="_blank">
    <img
      src="api_logo_pwrdBy_strava.png"
      alt="unterstützt von Strava"
      height="20"
    />
  </a>

  <Heading tag="h3">Pflege dein Bike für eine sorgenfreie Fahrt</Heading>
  <P
    >Behalte im Blick, wie lange deine Teile halten und wann sie gewartet werden
    müssen</P
  >
  <Heading tag="h5">Allgemeine Nutzung</Heading>
  <P>
    TendaBike lädt alle deine Aktivitäten und Fahrräder von Strava.
    <br /> Du kannst Teile anlegen und an deinen Fahrrädern anbauen. Du kannst
    Teile zwischen Fahrrädern verschieben.
    <br /> TendaBike berechnet die Nutzung für jedes Teil, das du anlegst.
    <br /> Du kannst außerdem Wartungspläne anlegen und Services eintragen. Du wirst
    erinnert, sobald ein Wartungsplan fällig ist.
  </P>
  <P>
    Wenn du TendaBike zum ersten Mal mit Strava verbindest, werden alle
    Fahrräder und Aktivitäten in die Datenbank geladen. Keine Sorge, du kannst
    später alles korrigieren. Änderungen auf Strava werden auch in TendaBike
    übernommen.
    <br /> Nach dem Verbinden kannst du Wartungspläne, Teile und Services hinzufügen.
    Am einfachsten geht das über die Detailseite eines Fahrrads.
  </P>

  <Heading tag="h5">Höhendaten</Heading>
  <P>
    Strava erfasst nur den Höhengewinn. Das ist ungünstig, wenn du Lifte oder
    Shuttles nutzt.
    <br />Bremsen verschleißen bergab deutlich schneller als bergauf, während z.
    B. Ketten eher beim Bergauffahren verschleißen.
    <br /> Wenn du ein aktuelles Garmin-Gerät und Garmin Connect nutzt, haben wir
    eine Lösung für dich!
  </P>
  <P>Das kannst du tun:</P>
  <P>
    <List tag="ol">
      <Li>
        Stoppe die Aufzeichnung in Liften und Shuttles. Dadurch sollte Strava
        den Höhengewinn des Shuttles nicht mehr erfassen.
      </Li>
      <Li>
        Lade die Aktivitätsliste von der Garmin-Connect-Website mit deutschen
        Spracheinstellungen herunter.
        <br /> Die resultierende Datei enthält den Höhenverlust.
      </Li>
      <Li>
        Lade diese Datei über '{m.header_sync()}' &rightarrow; '{m.header_csv()}'
        in TendaBike hoch.
      </Li>
    </List>
  </P>
  <Hr />
  <P>
    <div class="flex justify-end">
      © Christoph Rohland 2019-2026 - Quellcode auf&MediumSpace;
      <a href="https://github.com/hcrohland/tendabike" title="Repository">
        GitHub
      </a>
    </div>
  </P>
</div>
