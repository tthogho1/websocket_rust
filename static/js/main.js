// client.js
const button = document.getElementById('connectButton');
const configuration = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    // 必要に応じてTURNサーバーも追加
  ]
};
const selectElement = document.getElementById('userselect');
const usersbtn = document.getElementById('users');

let socket ;
let pc = null;
let candidates = [];
let stream;
let remoteStream;
//const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });

button.addEventListener('click', async () => {
  const currentUrl = window.location.href;
  // URLオブジェクトを作成
  const url = new URL(currentUrl);
  
  // プロトコルを判定（https -> wss, http -> ws）
  const wsProtocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  
  // ホスト名を取得
  const hostname = url.hostname;

  // WebSocketのURLを構築
  const userId = document.getElementById('user_id').value;
  const wsUrl = `${wsProtocol}//${hostname}/ws?name=${userId}`;

  socket = new WebSocket(wsUrl);

  socket.onopen = () => {
    console.log('WebSocket接続が確立しました');
    const textarea = document.getElementById('textarea');
    textarea.value += 'WebSocket接続が確立しました';

    registerUser(userId, 10, 10);
  };
    
  socket.onmessage = async (event) => {
    let jsondata;
    try{
      jsondata = JSON.parse(event.data);
    }catch(e){
      console.log(`event.data ${event.data}`);
      console.log(e);
    } 

    const data = jsondata.message;
    switch(data.type) {
      case 'offer':
        const user_id = jsondata.user_id;
        if (!confirm(`${user_id}からの接続を受け付けますか?`)) {
           return; 
        } 
        document.getElementById('to_id').value = user_id;
        await setupAndRespondToSdp(data.sdp);
        break;
      case 'answer':
        if (pc.signalingState !== 'stable') {
          //console.log(data.sdp);
          await pc.setRemoteDescription({ type: 'answer', sdp: data.sdp }).then(()=>{
              console.log('set answer to remote description');
            }
          ).catch( 
            error => console.log(error) 
          );
        }else{
          console.log('Connection already in stable state, skipping setRemoteDescription');
        }
        break;
      case 'ice':

        if (pc == null){
          console.log('pc is null');
          return;
        }
        await pc.addIceCandidate(data.candidate).then(()=>
          {
            console.log('add ice candidate from server')
          }
        ).catch(
          error => console.log(error)
        );
        break;
    }
  };

});

usersbtn.addEventListener('click', async () => {
  const userName = "";

  fetch('/users', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify({
        name: userName
    })
  })
  .then(response => response.json())
  .then(data => {
    const users = data.users;
    users.forEach((user) => {
      addOption(user.name);
    })
  })
  .catch((error) => {
      console.error('Error:', error);
  });
});

async function setupAndRespondToSdp(sdpMessage) {
  pc = new RTCPeerConnection(configuration);

  pc.ontrack = (event) => {
    console.log('ontrack');

    remoteStream = event.streams[0];
    const remoteVideoElement = document.getElementById('remote-video');
    if (remoteVideoElement) {
      remoteVideoElement.srcObject = remoteStream;
    } else {
      console.error('リモートビデオ要素が見つかりません');
    }
  };

  // ビデオストリームの処理
  stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
  stream.getTracks().forEach(
    track => {
      console.log('Adding track:', track.kind),
      pc.addTrack(track, stream)
    }
  );

  // ビデオ要素への表示
  const videoElement = document.getElementById('local-video');
  videoElement.srcObject = stream;

  //  OfferをRemoteに登録
  console.log("get offer from server");
  await pc.setRemoteDescription({ type: 'offer', sdp: sdpMessage })
  .then( 
    console.log("set remote description") 
  )
  .catch( 
    error => console.log(error) 
  );
  
  const offer = await pc.createAnswer();
  await pc.setLocalDescription(offer)
  .then(
    console.log("set local description")
  ).catch(
    error => console.log(error)
  );

  // ICE Candidateの処理
  pc.onicecandidate = (event) => {
    if (event.candidate) {
      console.log('onicecandidate');
      candidates.push(event.candidate);

      const messageObject = {
        user_id : document.getElementById('user_id').value,
        to_id : document.getElementById('to_id').value,
        message: { type: 'ice', candidate: event.candidate }
      }
      socket.send(JSON.stringify(messageObject));
    }
  }; 

  const messageObject = {
      user_id : document.getElementById('user_id').value,
      to_id : document.getElementById('to_id').value,
      message: { type: 'answer', sdp: offer.sdp }
  };
  console.log("send answer");
  socket.send(JSON.stringify(messageObject));

  monitorConnection(pc);

}

function monitorConnection(pc){
  const textarea = document.getElementById('textarea');

  // 接続状態の監視
  pc.onconnectionstatechange = () => {
    console.log('connection state change', pc.connectionState);
    textarea.value += `connection state change  ${pc.connectionState}`;
  };

  pc.oniceconnectionstatechange = () => {
    console.log('ICE connection state:', pc.iceConnectionState);
    textarea.value += `ICE connection state: ${pc.iceConnectionState}`;
  };

  pc.onsignalingstatechange = () => {
    console.log('signaling state:', pc.signalingState);
    textarea.value += `signaling state: ${pc.signalingState}`;
  }; 
}


async function createAndSendOffer() {
  try {
    pc = new RTCPeerConnection(configuration);
        // リモートストリームの処理
    pc.ontrack = (event) => {
      console.log('ontrack');

      const remoteStream = event.streams[0];
      const remoteVideoElement = document.getElementById('remote-video');
      if (remoteVideoElement) {
        remoteVideoElement.srcObject = remoteStream;
      }else{
        console.error('リモートビデオ要素が見つかりません');
      }
    };
    
    stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
    stream.getTracks().forEach(
      track => {
        console.log('Adding track:', track.kind);
        pc.addTrack(track, stream)
      }
    );

    // ローカルビデオの表示
    const localVideoElement = document.getElementById('local-video');
    if (localVideoElement) {
      localVideoElement.srcObject = stream;
    }

    // オファーの作成
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    // ICE Candidateの処理
    pc.onicecandidate = (event) => {
      console.log('onicecandidate');

      if (event.candidate) {
        const messageObject = {
          user_id : document.getElementById('user_id').value,
          to_id : document.getElementById('to_id').value,
          message: { type: 'ice', candidate: event.candidate }
        }
        socket.send(JSON.stringify(messageObject));
        candidates.push(event.candidate);
      }
    };

    const messageObject = {
      user_id : document.getElementById('user_id').value,
      to_id : document.getElementById('to_id').value,
      message: { type: 'offer', sdp: pc.localDescription.sdp }
    };
    socket.send(JSON.stringify(messageObject));

    monitorConnection(pc);

    return pc;
  } catch (error) {
    console.error('オファーの作成中にエラーが発生しました:', error);
  }
}


// UI要素（ビデオ要素など）を追加してください
document.addEventListener('DOMContentLoaded', () => {
  // pc = new RTCPeerConnection(configuration);

  // UIの初期化
  const offerButton = document.getElementById('createOfferButton');
  offerButton.addEventListener('click', async () => {
    await createAndSendOffer();
  });

  const closeButton = document.getElementById('closeButton');
  closeButton.addEventListener('click', () => {
    console.log('close');
    pc.close();
    pc = null;
    if (stream) {
      stream.getTracks().forEach(track => track.stop());
      stream = null;

    }
    if (remoteStream) {
      remoteStream.getTracks().forEach(track => track.stop());
      remoteStream = null;
    }
  });

  selectElement.onchange = () => {
    const selectedValue = selectElement.value;
    document.getElementById('to_id').value = selectedValue;
  };

});


function registerUser(username, latitude, longitude) {
  const data = {
    name:username,
    location:{
        lat: latitude,
        lng: longitude
    }
  };

  fetch('/position', {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify(data)
  })
  .then(response => response.json())
  .then(result => {
      textarea.value = JSON.stringify(result, null, 2);
  })
  .catch(error => {
      textarea.value = '送信エラー: ' + error.message;
  });
}

// optionを追加する関数
function addOption(text) {
  const option = new Option(text, text);
  selectElement.add(option);
};
