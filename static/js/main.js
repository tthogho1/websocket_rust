// client.js
const button = document.getElementById('connectButton');
const configuration = {
  iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },
    // 必要に応じてTURNサーバーも追加
  ]
};

let socket ;
let pc ;
let candidates = [];
let stream;
let remoteStream;
//const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });

button.addEventListener('click', async () => {
  const userId = document.getElementById('user_id').value;
  socket = new WebSocket(`ws://localhost:8000/ws?name=${userId}`);

  socket.onopen = () => {
    console.log('WebSocket接続が確立しました');
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

  // 接続状態の監視
  pc.onconnectionstatechange = () => {
    console.log('connection state change', pc.connectionState);
  };

  pc.oniceconnectionstatechange = () => {
    console.log('ICE connection state:', pc.iceConnectionState);
  };
  pc.onsignalingstatechange = () => {
    console.log('signaling state:', pc.signalingState);
  }; 
}

// UI要素（ビデオ要素など）を追加してください
document.addEventListener('DOMContentLoaded', () => {
  // UIの初期化
  const offerButton = document.getElementById('createOfferButton');
  offerButton.addEventListener('click', async () => {
    await createAndSendOffer();
  });

/*  const candidateSendButton = document.getElementById('sendCandidateButton');
  candidateSendButton.addEventListener('click', async () => {
    console.log('send candidate');

    candidates.forEach(candidate => {
      const messageObject = {
        user_id : document.getElementById('user_id').value,
        to_id : document.getElementById('to_id').value,
        message: { type: 'ice', candidate: candidate }
      }
      socket.send(JSON.stringify(messageObject));
    }) 
  }); */

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

});


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

    pc.onconnectionstatechange = () => {
      console.log('connection state change',pc.connectionState);
    };

    pc.oniceconnectionstatechange = () => {
      console.log('ICE connection state:', pc.iceConnectionState);
    };

    pc.onsignalingstatechange = () => {
      console.log('signaling state:', pc.signalingState);
    };

    return pc;
  } catch (error) {
    console.error('オファーの作成中にエラーが発生しました:', error);
  }
}