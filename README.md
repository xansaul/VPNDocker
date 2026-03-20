# VPNDocker Project

## Reporte y Descripción General

Este repositorio contiene la implementación de un sistema distribuido simple (Hub-Cliente) utilizando Rust, desplegado sobre una red privada virtual (VPN) con WireGuard y orquestado mediante Docker.

El objetivo es demostrar la comunicación segura y aislada entre contenedores en diferentes nodos de una red, simulando un entorno distribuido real.

> **Reporte Completo**: Para detalles teóricos, arquitectura y análisis del proyecto, consulte los documentos: [EntregaAvancesTernurinesTecnologicos.pdf](docs/reportes/EntregaAvancesTernurinesTecnologicos.pdf) y [EntregaFinalTernurinesTecnologicos.pdf](docs/reportes/EntregaFinalTernurinesTecnologicos.pdf).

## Requisitos de Software

Para ejecutar este proyecto, necesitará tener instalado:

1.  **Docker** y **Docker Compose**: Para la orquestación de contenedores.
2.  **WireGuard**: Debe estar instalado en el sistema host para manejar las interfaces de red VPN. Nota: Con WireGuard solo se puede usar en Linux por la propiedad `network_mode: host`, pero el proyecto se puede levantar en cualquier sitio con el archivo `rust/mandelbrot-dist/docker-compose.emulated.yml`, creando también la red indicada en el mismo (`docker network create distnet`). Lea las Notas Importantes y Supuestos para más detalles.
3.  **Rust & Cargo** (Opcional): Si desea compilar y ejecutar los binarios fuera de Docker.

## Instrucciones VPN (WireGuard)

El sistema asume que los nodos están conectados a través de una VPN WireGuard.

### Generar Claves (One-liner)

Para generar las claves privada y pública rápida y fácilmente en un one-liner, ejecuta:
```bash
wg genkey | tee privatekey | wg pubkey > publickey
```

### Configuración del Hub
La configuración para el Hub (`wg0.conf`) debe verse así:

```ini
[Interface]
Address = 10.10.10.1/24 
ListenPort = 51820
PrivateKey = <private_key>

# Opcional: Para que la VM actúe como servidor y dé internet a otros
PostUp = iptables -A FORWARD -i %i -j ACCEPT; iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
PostDown = iptables -D FORWARD -i %i -j ACCEPT; iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE

[Peer]
PublicKey = <public_key>
AllowedIPs = 10.10.10.2/32

[Peer]
PublicKey = <public_key>
AllowedIPs = 10.10.10.3/32
```

### Configuración del Cliente (Peer)
Ejemplo de `wg0.conf` para un cliente (Peer):

```ini
[Interface]
Address = 10.10.10.2/24
PrivateKey = <private_key>

[Peer]
PublicKey = <public_key>
Endpoint = <public_ip>:51820
AllowedIPs = 10.10.10.1/32, 10.10.10.0/24
PersistentKeepalive = 25
```

### Levantar la VPN
Para cada nodo (Hub o Cliente), utilice el archivo de configuración correspondiente en `vpn/<nodo>/wg0.conf`:

```bash
# Ejemplo para levantar la interfaz (requiere privilegios de root/admin)
wg-quick up ./vpn/hub/wg0.conf
# O para un cliente
wg-quick up ./vpn/peer/wg0.conf
```

Verifique la conexión:
```bash
wg show
ping 10.10.10.1  # Desde un cliente al Hub
```

## Despliegue con Docker

Para la construcción de las imágenes y la ejecución, **se debe estar en la carpeta** `rust/mandelbrot-dist`:

```bash
cd rust/mandelbrot-dist
```

> **Nota:** Se deben ejecutar los comandos de `docker-compose` iguales como están listados a continuación.

### Nodo HUB
En la máquina que actuará como servidor:

```bash
docker-compose -f docker-compose.hub.yml up -d --scale cliente_local=4
```

> Si usa el compose del hub (`docker-compose.hub.yml`), **no se necesita configurar el archivo `.env`**.

Esto iniciará el servicio `hub` escuchando en el puerto `7878` sobre la red del host (accesible vía VPN IP `10.10.10.1`).

### Nodo Cliente
En las máquinas cliente:

1.  Asegúrese de que la VPN esté activa y pueda hacer ping al Hub.
2.  Si usa el compose del cliente (`docker-compose.client.yml`), **se debe copiar y renombrar** el archivo de ejemplo a `.env` y llenar las variables correspondientes:
    ```bash
    cp .env.example .env
    # Edita .env con tus variables
    ```
3.  Despliegue el cliente:

```bash
# Puede configurar la IP del HUB si es diferente por defecto usando variables de entorno o editando el docker-compose
docker-compose -f docker-compose.client.yml up -d --scale cliente_local=4
```

El cliente intentará conectar a `HUB_ADDR` (definido en el archivo `.env`, por defecto `10.10.10.1:7878`).

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
-   **Emulación**: Para emular múltiples nodos en una sola máquina, también debe ubicarse en la carpeta `rust/mandelbrot-dist`. Primero se necesita crear manualmente una red de Docker llamada "distnet" ejecutando `docker network create distnet`, y luego se puede usar `docker-compose -f docker-compose.emulated.yml up -d --scale worker=4`.
-   **Volúmenes (Imágenes Generadas)**: El contenedor del Hub mapea la ruta local `./output` al directorio `/usr/local/bin/output` dentro del contenedor. De esta forma, todas las imágenes fractales que se generen persistirán de manera segura en la carpeta `rust/mandelbrot-dist/output` de su máquina host real.

## Interacción con la API REST

Para probar el sistema de generación de fractales, puedes interactuar con su API REST (usualmente expuesta en el puerto `8080` del Hub, por ejemplo `http://localhost:8080`).

### Rutas Disponibles

-   `GET /health`: Verifica el estado de salud del Hub.
-   `POST /jobs`: Crea un nuevo trabajo para generar una imagen fractal.
-   `GET /jobs`: Lista todos los trabajos y sus resúmenes.
-   `GET /jobs/:id`: Obtiene el estado detallado de un trabajo específico.
-   `GET /gallery`: Muestra una galería interactiva en HTML con las imágenes generadas.
-   `GET /images/<filename>`: Sirve los archivos de las imágenes estáticas directamente desde el directorio de resultados.

### Ejemplo: Crear un Job (Generar Imagen)

Crear un *Job* es equivalente a comandar la creación de una imagen. La solicitud requiere cierta información matemática y configuración en el cuerpo de la petición. Puedes lanzar una petición `POST` a la ruta `/jobs` con la siguiente información de prueba (por ejemplo usando Postman, Thunder Client o `curl`):

**Opción 1:**
```bash
curl -X POST http://localhost:8080/jobs \
-H "Content-Type: application/json" \
-d '{
  "img_width": 1200,
  "img_height": 1200,
  "max_iter": 8500,
  "x_start": -0.748,
  "x_end": -0.744,
  "y_start": 0.1,
  "y_end": 0.104
}'
```

**Opción 2:**
```bash
curl -X POST http://localhost:8080/jobs \
-H "Content-Type: application/json" \
-d '{
  "img_width": 1200,
  "img_height": 1200,
  "max_iter": 250,
  "x_start": -2.0,
  "x_end": 0.5,
  "y_start": -1.25,
  "y_end": 1.25
}'
```