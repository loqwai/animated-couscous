# animated-couscous


## Install

### Ubuntu

sudo apt-get update && apt install build-essential g++ pkg-config libx11-dev libasound2-dev libudev-dev


## Networking Plans

The system has 6 essential jobs:

1. Render a given state
2. Send inputs to the network
3. Receive inputs from the network
4. Calculate the next state
5. Send state to the network
6. Receive state from the network

My current plan is to put each of these components into their own Bevy Plugin. They will communicate with each other using EventReader/EventWriters. I'm currently planning on having all clients run 1, 2, and 6. The server will do 3, 4, & 5.

Only job 6 will be allowed to modify any ECS components and it must do so in a single synchronous thread. The other jobs will only be allowed to read ECS components.

Job 4 will maintain the state outside of Bevy components. The state will be network serializable so that it can be easily sent to the clients.

For now, the application will take a CLI flag to enable the server. In the future, we may want to have all clients run the server and elect a leader, or have them run in "lockstep" and reconcile differences using a CRDT.
