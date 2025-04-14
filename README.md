# WebSocket Rust - Real-time Location-based Chat Application

A real-time web application that connects users on Google Maps with text chat and video chat functionality. This application allows users to see each other's locations on a map and communicate in real-time.

## Features

- **Real-time User Location Tracking**: Users' locations are displayed on Google Maps
- **Text Chat**: Real-time messaging between users
- **Video Chat**: WebRTC-based video communication
- **Proximity-based User Discovery**: Find users within a specific radius
- **Persistent User Data**: User information is stored in Redis

## Technology Stack

- **Backend**: Rust with Axum web framework
- **Real-time Communication**: WebSockets for chat and WebRTC for video
- **Database**: Redis for storing user data and geospatial information
- **Frontend**: React with TypeScript, Tailwind CSS, and Google Maps integration

## Prerequisites

- Rust (latest stable version)
- Node.js and npm (for the React frontend)
- Redis server
- Environment variables setup (see Configuration section)

## Configuration

### Backend Configuration

The backend requires the following environment variables:

- `REDIS_URL`: URL for connecting to Redis
- `PORT`: Port number for the web server
- `WS_SERVER`: WebSocket server URL

You can set these in a `.env` file in the project root.

Example `.env` file:
```
REDIS_URL=redis://localhost:6379
PORT=3000
WS_SERVER=ws://localhost:3000/ws
```

### Frontend Configuration

The React frontend requires the following environment variables:

- `REACT_APP_GOOGLE_MAPS_API_KEY`: Google Maps API key
- `REACT_APP_WEBSOCKET_URL`: WebSocket server URL

You can set these in a `.env` file in the `front` directory.

Example `.env` file for frontend:
```
REACT_APP_GOOGLE_MAPS_API_KEY=your_google_maps_api_key
REACT_APP_WEBSOCKET_URL=ws://localhost:3000
```

## Installation

### Backend Installation

1. Clone the repository
2. Install Rust dependencies:
   ```
   cargo build
   ```
3. Set up environment variables (see Backend Configuration section)
4. Run the backend:
   ```
   cargo run
   ```

### Frontend Installation

1. Navigate to the `front` directory:
   ```
   cd front
   ```
2. Install Node.js dependencies:
   ```
   npm install
   ```
3. Set up environment variables (see Frontend Configuration section)
4. Run the frontend development server:
   ```
   npm start
   ```

## Usage

1. Open a web browser and navigate to `http://localhost:<PORT>/Chat`
2. Allow location access when prompted
3. Your position will be displayed on the map
4. Other users in the vicinity will appear on the map
5. Click on a user to initiate a chat or video call

## API Endpoints

- `/ws`: WebSocket endpoint for real-time communication
- `/Chat`: Main application page
- `/position`: POST endpoint to update user position
- `/users`: POST endpoint to get all users
- `/usersinbounds`: POST endpoint to get users within a specific radius

## How It Works

### Backend Flow

1. **User Registration**: When a user connects, their location is stored in Redis
2. **Location Updates**: Users' positions are updated in real-time and broadcast to other users
3. **Chat Messages**: Text messages are sent through WebSockets and delivered to the appropriate recipients
4. **Video Chat**: WebRTC is used to establish peer-to-peer connections for video communication
5. **User Removal**: When a user disconnects, their information is removed from Redis and other users are notified

### Frontend Flow

1. **Authentication**: Users log in using AWS Cognito authentication
2. **Map Initialization**: Google Maps is initialized with the user's current location
3. **WebSocket Connection**: A WebSocket connection is established to receive real-time updates
4. **User Discovery**: Nearby users are displayed on the map based on geolocation
5. **Chat Functionality**: 
   - Users can send messages to specific users or broadcast to all
   - Messages are transmitted via WebSockets
   - The chat interface displays incoming and outgoing messages
6. **Video Chat**:
   - Users can initiate video calls with other users
   - WebRTC is used for peer-to-peer video/audio communication
   - The application handles SDP (Session Description Protocol) and ICE (Interactive Connectivity Establishment) for WebRTC connection setup
   - Users can accept or decline incoming video call requests

## Development

### Backend Structure

The backend project structure follows standard Rust conventions:

- `src/main.rs`: Application entry point and server setup
- `src/lib.rs`: Core data structures and utility functions
- `src/handlers/`: Request handlers for different endpoints
  - `position.rs`: Handlers for location-related endpoints
  - `websocket.rs`: WebSocket connection handling

### Frontend Structure

The frontend is a React application with TypeScript and is located in the `front` directory:

- `src/App.tsx`: Main application component with routing
- `src/components/`: React components
  - `GoogleMap.tsx`: Main map component
  - `MapCtrl.tsx`: Map control component
  - `videoChat.tsx`: Video chat component
  - `WebSocketProvider.tsx`: WebSocket context provider
  - `parts/`: Smaller UI components
- `src/contexts/`: React context providers
  - `AuthContext.tsx`: Authentication context
- `src/api/`: API communication functions
- `src/types/`: TypeScript type definitions
- `src/util/`: Utility functions

### Key Frontend Components

#### Authentication Components
- **Login**: Handles user authentication using AWS Cognito
- **SignUp**: Allows new users to register
- **AuthContext**: Manages authentication state throughout the application

#### Map Components
- **GoogleMap**: Main component that initializes the Google Maps API and gets the user's current location
- **MapCtrl**: Controls the map display, handles user markers, and manages the map's idle state to fetch nearby users
- **MarkerWithInfoWindow**: Displays user markers on the map with information windows

#### Communication Components
- **WebSocketProvider**: Manages WebSocket connections for real-time communication
- **SendMsgForm**: Interface for sending and receiving text messages
- **VideoChat**: Handles video chat functionality using WebRTC
  - Manages media streams
  - Handles SDP offer/answer exchange
  - Manages ICE candidate exchange
  - Provides UI controls for video chat (connect/disconnect, etc.)

#### Utility Classes
- **WebSocketWrapper**: Wraps WebSocket functionality
- **WebRtc**: Manages WebRTC peer connections
- **EventBus**: Provides event-based communication between components

## Frontend-Backend Integration

The frontend and backend integrate through several key mechanisms:

### WebSocket Communication
- The React frontend establishes a WebSocket connection to the Rust backend
- Real-time messages are exchanged through this connection
- The backend broadcasts messages to appropriate recipients
- The frontend displays incoming messages and allows sending new ones

### REST API Calls
- The frontend makes HTTP requests to the backend API endpoints
- User positions are updated via the `/position` endpoint
- Nearby users are fetched via the `/usersinbounds` endpoint
- The backend processes these requests and interacts with Redis

### WebRTC Signaling
- The WebSocket connection is used for WebRTC signaling
- SDP offers/answers and ICE candidates are exchanged through WebSockets
- The backend acts as a relay for WebRTC signaling messages
- Once the WebRTC connection is established, video/audio data flows directly between peers

### User Management
- When users connect to the frontend, their information is sent to the backend
- The backend stores user data in Redis, including geolocation
- When users disconnect, the backend removes their data and notifies other users
- The frontend updates the map display based on user presence/absence notifications

## License

[MIT License](LICENSE)
