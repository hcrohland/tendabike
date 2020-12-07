<script lang="ts">
  import {message} from './store'
  import {Container, Row, Col, Toast, ToastBody, ToastHeader} from 'sveltestrap'

  function toggle() {
    let m = $message
    m.active= false
    message.set(m)
  }
</script>
{#if $message.active}
<Container>
  <Row class="justify-content-center">
    <Col xs="auto">
      <div class="p-2 bg-danger mb-2">
        <Toast  class="mr-1">
          <ToastHeader {toggle}>
            {$message.status}
          </ToastHeader>
          <ToastBody>
            {@html $message.message}
          </ToastBody>
        </Toast> 
      </div>
    </Col>
  </Row>
</Container>
{/if}
