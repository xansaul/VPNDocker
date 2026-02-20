# VPNDocker Project

## Reporte y Descripción General

Este repositorio contiene la implementación de un sistema distribuido simple (Hub-Cliente) utilizando Rust, desplegado sobre una red privada virtual (VPN) con WireGuard y orquestado mediante Docker.

El objetivo es demostrar la comunicación segura y aislada entre contenedores en diferentes nodos de una red, simulando un entorno distribuido real.

> **Reporte Completo**: Para detalles teóricos, arquitectura y análisis del proyecto, consulte el documento: [EntregaAvancesTernurinesTecnologicos.pdf](docs/reportes/EntregaAvancesTernurinesTecnologicos.pdf).

## Requisitos de Software

Para ejecutar este proyecto, necesitará tener instalado:

1.  **Docker** y **Docker Compose**: Para la orquestación de contenedores.
2.  **WireGuard**: Debe estar instalado en el sistema host para manejar las interfaces de red VPN si se usa `network_mode: host` (Linux) o para conectar los nodos (Windows/Mac).
3.  **Rust & Cargo** (Opcional): Si desea compilar y ejecutar los binarios fuera de Docker.

## Instrucciones VPN (WireGuard)

El sistema asume que los nodos están conectados a través de una VPN WireGuard con las siguientes asignaciones de IP (ejemplo):

-   **Hub**: `10.10.10.1`
-   **Clientes**: `10.10.10.x` (ej. `10.10.10.2`)

### Levantar la VPN
Para cada nodo (Hub o Cliente), utilice el archivo de configuración correspondiente en `vpn/<nodo>/wg0.conf`:

```bash
# Ejemplo para levantar la interfaz (requiere privilegios de root/admin)
wg-quick up ./vpn/hub/wg0.conf
# O para un cliente
wg-quick up ./vpn/chino/wg0.conf
```

Verifique la conexión:
```bash
wg show
ping 10.10.10.1  # Desde un cliente al Hub
```

## Despliegue con Docker

### Nodo HUB
En la máquina que actuará como servidor:

```bash
docker-compose -f docker/docker-compose.hub.yml up --build
```

Esto iniciará el servicio `hub` escuchando en el puerto `7878` sobre la red del host (accesible vía VPN IP `10.10.10.1`).

### Nodo Cliente
En las máquinas cliente:

1.  Asegúrese de que la VPN esté activa y pueda hacer ping al Hub.
2.  Despliegue el cliente:

```bash
# Puede configurar la IP del HUB si es diferente por defecto usando variables de entorno o editando el docker-compose
docker-compose -f docker/docker-compose.client.yml up --build
```

El cliente intentará conectar a `HUB_ADDR` (definido en el docker-compose, por defecto `10.10.10.1:7878`).

## Compilación y Ejecución Manual (Rust)

Si prefiere ejecutar los binarios directamente sin Docker:

1.  Navegue al directorio del proyecto Rust:
    ```bash
    cd rust/connection-tcp
    ```

2.  **Ejecutar Hub**:
    ```bash
    cargo run --bin hub
    ```

3.  **Ejecutar Cliente**:
    ```bash
    # Asegúrese de configurar la dirección del servidor si es necesario
    export SERVER_ADDR=127.0.0.1:7878
    cargo run --bin client
    ```

## Notas Importantes y Supuestos

-   **Red Host**: Los archivos `docker-compose` utilizan `network_mode: "host"`. Esto es ideal en Linux para que los contenedores usen directamente la interfaz WireGuard (`wg0`) del host. En Windows/Mac, Docker Desktop tiene aislamiento de red, por lo que podría requerir configuración adicional de puertos o no funcionar exactamente igual con `host` mode.
-   **Firewall**: Asegúrese de que el puerto `51820/udp` (WireGuard) y `7878/tcp` (Aplicación) estén permitidos en el firewall de los nodos.
-   **Configuración**: Las claves privadas en los archivos `wg0.conf` deben ser manejadas con seguridad. Los archivos provistos pueden contener claves de ejemplo o placeholders.
