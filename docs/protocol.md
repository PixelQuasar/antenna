## Базовая информация

Antenna - SDP for building decentralized P2P over WebRTC. SDK спроектирован быть максимально гибким для пользователя, используя при этом преимущества протокола webRTC, при этом полностью инкапсулируя его API и решая насущные задачи вроде сигналинга и реконнекта самостоятельно. Antenna кросплатформенный SDK и может быть использован как в браузерных, так и в нативных приложениях, хоть фокус на браузере. SDK построен на Antenna-protocol, о котором мы поговорим далее. 

## Identity

## Discovery

## Peer structure

Пир состоит из двух модулей: FSM и Driver. 

FSM является синхронным обработчиком (конечным автоматом) и отвечает за всю протокольную логику и управление состоянием подключений. Является платформонезависимым. 

Driver является платформеннозависимой прослойкой и обрабатывает ввод пользователя, обеспечивает вывод и управляет внешними API: webRTC, персистентного хранилища. Driver реализован отдельно для каждой из платформ: в данный момент поддерживается web и native.

### FSM

### Driver

## Handshake

Для установки соединения между пирами необходимо провести хандшейк. В ходе antenna-хандшейка решается две проблемы: обмен SDP-контрактами и распознавание IDENTITY друг друга (TODO написать подробнее).

### SDP-negotiation

### Identity negotiation

Хандшейк спроектирован так, чтобы минимизировать делегацию сигналинг-части на юзера, но при этом не завязываться на конкретном инструменте сигналинга. В ходе хандшейка два пира выбирают свои роли: Host и Joiner. В ходе хандшейка они обмениваются информацией друг о друге, что позволяет установить соединение - offer, answer. Host отправляет offer, а joiner принимает его и отправляет answer, после чего host initiates peer connection Абстракто, хэндшейк представляет из себя следующую картину:

```mermaid
sequenceDiagram

    participant HOST
    participant JOINER

    HOST ->> JOINER: offer
    JOINER ->> HOST: answer
    HOST <<-->> JOINER: establishing connection

```


Ханшдейк бывает двух видов: Bootstrap и Relay. TODO кратко описать различия

### Bootstrap

Подробная схема bootstrap-соединения:

```mermaid
sequenceDiagram
    participant A
    participant AD 
    participant BD
    participant B

    alt await open SDP offer
        AD ->> A: Input::InitOpenOffer
        A ->> AD: Output::InitOpenOffer
        AD ->> A: Input::OpenOfferCreated
        A ->> A: write offer to metadata containing SDP, public key and biscuit token
    end
    AD ->> BD: get offer from FSM metadata and transfer it to B client somehow
    
    alt await SDP answer
        BD ->> B: Input::InitHandshake bootstrap
        BD ->> B: HandshakeInput::Offer
        B ->> BD: HandshakeOutput::RequestSDPAnswer
        BD ->> B: HandshakeInput::AnswerCreated
        B ->> B: write answer to metadata containing SDP, public key and biscuit token
    end

    BD ->> AD: get answer from FSM metadata and transfer it back to A
    AD ->> A: HandshakeInput::Answer
    A ->> AD: HandshakeOutput::AcceptSDPAnswer
    AD <<->> BD: webRTC onOpen callback invocation, DC established    
    note over A: connected & available
    note over B: connected & available
```

### Relay

Подробная схема relay-соединения:

```mermaid
sequenceDiagram
    participant A
    participant AD 
    participant BD
    participant B
    participant CD
    participant C

    note over A: connected & available
    note over B: connected & available
    A <<-->> B: already connected and established DC
    B <<->> C: establishing bootstrap connection
    note over C: connected
   
    B ->> C: Msg RelayPayload::InitJoiner
    C ->> C: Input::InitHandshake relay via B
    B ->> A: Msg RelayPayload::InitHost
    A ->> A: Input::InitHandshake relay via B
    A <<-->> C: handshake established
    A ->> A: HandshakeInput::Init
    A ->> AD: HandshakeOutput::InitSDPOffer
    AD ->> A: HandshakeInput::OfferCreated

    A ->> B: RelayTo(RelayPayload::Offer)
    B ->> C: RelayFrom(RelayPayload::Offer)

    C ->> C: HandshakeInput::Offer
    C ->> CD: HandshakeOutput::RequestSDPAnswer
    CD ->> C: HandshakeInput::AnswerCreated

    C ->> B: RelayTo(RelayPayload::Answer)
    B ->> A: RelayFrom(RelayPayload::Answer)

    A ->> A: HandshakeInput::Answer
    A ->> AD: HandshakeOutput::AcceptSDPAnswer
    AD <<->> CD: webRTC onOpen callback invocation, DC

    note over C: available
```


Важно отметить, что после прохождения хэндшейка и установки PeerConnection, иерархия между пирами "хост-джоинер" пропадает и пиры становятся полностью равноправными.

## Peer connection

### Message format

Тип Antenna-сообщения полностью user-defined и должен лишь реализовывать Deserialize, Serialize и Clone, чтобы свободно работать в рамках соединения. Сам способ сериализации определяется юзером. 