const socket = new WebSocket(`ws://${location.host}/socket`);
//socket.addEventListener('close', () => {
//    document.getElementById("iframe").hidden = true;
//    window.close();
//});
socket.addEventListener('message', event => {
    document.getElementById("line").innerText = event.data;
    const state = JSON.parse(event.data);
    const frame = document.getElementById("iframe");
    switch (state.state) {
        case "Closed":
            frame.src = "https://op.gg";
            break;
        case "Idle":
            frame.src = `https://op.gg/summoners/${state.info.server}/${state.info.username}`;
            break;
        case "ChampSelect":
            frame.src = `https://op.gg/multisearch/${state.info.server}?summoners=${encodeURIComponent(state.additional_info.join(','))}`;
            console.log(frame.src)
            break;
        case "InGame":
            frame.src = `https://op.gg/summoners/${state.info.server}/${state.info.username}/ingame`;
            break;
    }
});