<script lang="ts">
  import { Heading, Hr, Li, List, P } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
</script>

<div class="p-4 sm:p-6 md:p-8 m-8 max-w-200">
  <a href="http://www.strava.com" class="float-end items-start" target="_blank">
    <img src="api_logo_pwrdBy_strava.png" alt="powered by Strava" height="20" />
  </a>

  <Heading tag="h3">Lueg zu dim Velo für no meh Fahrfreud</Heading>
  <P>Behalt im Blick, wie lang dini Teili halte und wänn sie Service bruuche.</P
  >

  <Heading tag="h5">Allgemein</Heading>
  <P>
    TendaBike holt alli dini Aktivitäte und Velos vo Strava.
    <br /> Du chasch Teili erfasse und dine Velos zueordne. Teili chönd au
    zwüsche de Velos verschobe werde.
    <br /> TendaBike berechnet d'Belastig für alli Teili, wo du erfasst hesch.
    <br /> Du chasch Serviceplän aalege, Service erfasse und wirsch erinnert, wenn
    en Service fällig isch.
  </P>

  <P>
    Bi de erschte Verbindig mit Strava lädt TendaBike alli Velos und Aktivitäte
    i d'Datebank. Kei Angscht – du chasch nachträglich alles korrigiere.
    Änderige uf Strava wärde automatisch i TendaBike übernoh.
    <br /> Nach em Verbinde chasch Serviceplän, Teili und Service erfasse. Am eifachschte
    geit das uf dr Detailsite vom Velo.
  </P>

  <Heading tag="h5">Höhemeter</Heading>
  <P>
    Strava speicheret nume de Ufstieg. Das isch unpraktisch, wenn du mit Lift
    oder Posti underwägs bisch.
    <br /> Bremse verschliissed zum Biispiel bi de Abfahrt viel stärker, während
    Chettene vor allem bi de Ufstieg beansprucht wärde.
    <br /> Falls du es neus Garmin-Grät und Garmin Connect bruchsch, hämmer au dafür
    e Lösig.
  </P>

  <P>So gaht's:</P>

  <P>
    <List tag="ol">
      <Li>
        Stopp d'Uufzeichnig während Lift- oder Postifahrte. So wird dä Höhegwünn
        vo dä Abschnitte nöd uf Strava übernoh.
      </Li>
      <Li>
        Lad dini Aktivitäte us Garmin Connect als CSV-Datei mit dütscher Sprach
        iistellig abe.
        <br /> Die exportierti Datei enthält au de Abstig.
      </Li>
      <Li>
        Lad d'Datei i TendaBike über '{m.header_sync()}' &rightarrow; '{m.header_csv()}'
        uuf.
      </Li>
    </List>
  </P>

  <Hr />

  <P>
    <div class="flex justify-end">
      © Christoph Rohland 2019–2026 – Quellcode uf
      <a href="https://github.com/hcrohland/tendabike" title="Repository">
        GitHub
      </a>
    </div>
  </P>
</div>
