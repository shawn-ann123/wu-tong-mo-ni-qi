#include "socket_platform.hpp"

#include <algorithm>
#include <cstring>
#include <limits>

namespace daedalus::sim::sdk::v1::detail {

ClientStatus ensureSocketRuntime() {
  return ClientStatus::success();
}

void closeSocket(SocketHandle socket) noexcept {
  if (socket == kInvalidSocket) return;
  ::close(socket);
}

void shutdownSocket(SocketHandle socket) noexcept {
  if (socket == kInvalidSocket) return;
  ::shutdown(socket, SHUT_RDWR);
}

std::string socketErrorMessage(const char* operation) {
  return std::string(operation) + " failed: " + std::strerror(errno);
}

ClientResult<ResolvedAddress> resolveIpv4(const UdpEndpoint& endpoint,
                                          int socket_type, int protocol) {
  if (endpoint.host.empty() || endpoint.port == 0) {
    return ClientResult<ResolvedAddress>::failure(
        ClientError::InvalidArgument, "endpoint host and port must be set");
  }
  auto runtime = ensureSocketRuntime();
  if (!runtime) {
    return ClientResult<ResolvedAddress>::failure(runtime.error,
                                                   runtime.message);
  }

  addrinfo hints{};
  hints.ai_family = AF_INET;
  hints.ai_socktype = socket_type;
  hints.ai_protocol = protocol;
  addrinfo* addresses = nullptr;
  const auto port = std::to_string(endpoint.port);
  const int result = getaddrinfo(endpoint.host.c_str(), port.c_str(), &hints,
                                 &addresses);
  if (result != 0 || addresses == nullptr) {
    return ClientResult<ResolvedAddress>::failure(
        ClientError::ResolveFailed,
        "failed to resolve " + endpoint.host + ":" + port);
  }

  ResolvedAddress resolved{};
  if (addresses->ai_addrlen > sizeof(resolved.storage)) {
    freeaddrinfo(addresses);
    return ClientResult<ResolvedAddress>::failure(
        ClientError::ResolveFailed, "resolved address is unexpectedly large");
  }
  std::memcpy(&resolved.storage, addresses->ai_addr, addresses->ai_addrlen);
  resolved.length = static_cast<socklen_t>(addresses->ai_addrlen);
  freeaddrinfo(addresses);
  return ClientResult<ResolvedAddress>::success(resolved);
}

ClientStatus setReceiveTimeout(SocketHandle socket, int timeout_ms) {
  if (timeout_ms < 0) {
    return ClientStatus::failure(ClientError::InvalidArgument,
                                 "timeout must be non-negative");
  }
  const timeval value{timeout_ms / 1000, (timeout_ms % 1000) * 1000};
  const int result = setsockopt(socket, SOL_SOCKET, SO_RCVTIMEO, &value,
                                sizeof(value));
  if (result != 0) {
    return ClientStatus::failure(ClientError::SocketCreateFailed,
                                 socketErrorMessage("setsockopt"));
  }
  return ClientStatus::success();
}

ClientStatus sendAll(SocketHandle socket, const std::uint8_t* data,
                     std::size_t size) {
  std::size_t sent = 0;
  while (sent < size) {
    const auto chunk = static_cast<int>(std::min<std::size_t>(
        size - sent, static_cast<std::size_t>(std::numeric_limits<int>::max())));
    const int result = ::send(
        socket, reinterpret_cast<const char*>(data + sent), chunk, 0);
    if (result <= 0) {
      return ClientStatus::failure(ClientError::SendFailed,
                                   socketErrorMessage("send"));
    }
    sent += static_cast<std::size_t>(result);
  }
  return ClientStatus::success();
}

ClientStatus receiveAll(SocketHandle socket, std::uint8_t* data,
                        std::size_t size) {
  std::size_t received = 0;
  while (received < size) {
    const auto chunk = static_cast<int>(std::min<std::size_t>(
        size - received,
        static_cast<std::size_t>(std::numeric_limits<int>::max())));
    const int result = ::recv(
        socket, reinterpret_cast<char*>(data + received), chunk, 0);
    if (result == 0) {
      return ClientStatus::failure(ClientError::PeerClosed,
                                   "peer closed the TCP stream");
    }
    if (result < 0) {
      return ClientStatus::failure(ClientError::ReceiveFailed,
                                   socketErrorMessage("recv"));
    }
    received += static_cast<std::size_t>(result);
  }
  return ClientStatus::success();
}

}  // namespace daedalus::sim::sdk::v1::detail
