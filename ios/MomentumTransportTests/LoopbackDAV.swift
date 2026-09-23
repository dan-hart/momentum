// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Network

/// A disposable HTTP peer inside the iOS test process. It accepts only loopback,
/// requires fixture credentials, and implements the ETag boundary used by Nextcloud.
/// All mutable state is main-actor isolated; Network callbacks only forward events.
@MainActor final class LoopbackDAV {
    private let listener: NWListener
    private let queue = DispatchQueue(label: "momentum.test.webdav")
    private var startup: CheckedContinuation<Void, any Error>?
    private var connections: [ObjectIdentifier: NWConnection] = [:]
    private var heldGET: (NWConnection, String)?
    private var heldPROPFIND: (NWConnection, String)?
    private var files: [String: Data] = [:]
    private var version = 0
    private var collectionExists = false
    private(set) var gets = 0
    private(set) var puts = 0
    private(set) var conflicts = 0
    private(set) var collections = 0
    private(set) var propfinds = 0
    var rejectAuthentication = false
    var conflictNextPUT = false
    var onHeldGET: (() -> Void)?
    var onHeldPROPFIND: (() -> Void)?
    var url: String { "http://127.0.0.1:\(listener.port!.rawValue)" }
    var payload: Data? { files.values.first }
    private var etag: String { "\"fixture-\(version)\"" }

    init() throws {
        let parameters = NWParameters.tcp
        parameters.requiredLocalEndpoint = .hostPort(host: "127.0.0.1", port: .any)
        listener = try NWListener(using: parameters)
    }

    func start() async throws {
        listener.newConnectionHandler = { [weak self] connection in
            Task { @MainActor in self?.accept(connection) }
        }
        try await withCheckedThrowingContinuation { continuation in
            startup = continuation
            listener.stateUpdateHandler = { [weak self] state in
                Task { @MainActor in
                    guard let self, let startup = self.startup else { return }
                    switch state {
                    case .ready: self.startup = nil; startup.resume()
                    case .failed(let error): self.startup = nil; startup.resume(throwing: error)
                    case .cancelled: self.startup = nil; startup.resume(throwing: CancellationError())
                    default: break
                    }
                }
            }
            listener.start(queue: queue)
        }
    }

    func stop() {
        listener.cancel()
        for connection in connections.values { connection.cancel() }
        connections.removeAll()
        heldGET = nil
        heldPROPFIND = nil
        onHeldGET = nil
        onHeldPROPFIND = nil
    }

    func releaseGET() {
        guard let (connection, path) = heldGET else { return }
        heldGET = nil
        get(connection, path: path)
    }

    func releasePROPFIND() {
        guard let (connection, path) = heldPROPFIND else { return }
        heldPROPFIND = nil
        propfind(connection, path: path)
    }

    private func accept(_ connection: NWConnection) {
        connections[ObjectIdentifier(connection)] = connection
        connection.start(queue: queue)
        receive(connection, buffered: Data())
    }

    private func receive(_ connection: NWConnection, buffered: Data) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, complete, error in
            Task { @MainActor in
                guard let self else { connection.cancel(); return }
                var buffer = buffered
                if let data { buffer.append(data) }
                guard buffer.count <= 4 * 1024 * 1024 else { self.close(connection); return }
                if let boundary = buffer.range(of: Data("\r\n\r\n".utf8)),
                   let head = String(data: buffer[..<boundary.lowerBound], encoding: .utf8) {
                    let lines = head.components(separatedBy: "\r\n")
                    var headers: [String: String] = [:]
                    for line in lines.dropFirst() {
                        if let colon = line.firstIndex(of: ":") {
                            headers[String(line[..<colon]).lowercased()] = line[line.index(after: colon)...].trimmingCharacters(in: .whitespaces)
                        }
                    }
                    let length = Int(headers["content-length"] ?? "0") ?? 0
                    if buffer.count - boundary.upperBound >= length {
                        let request = lines[0].split(separator: " ")
                        guard request.count == 3 else { self.close(connection); return }
                        self.handle(connection, method: String(request[0]), path: String(request[1]), headers: headers,
                                    body: Data(buffer[boundary.upperBound..<(boundary.upperBound + length)]))
                        return
                    }
                }
                if complete || error != nil { self.close(connection) }
                else { self.receive(connection, buffered: buffer) }
            }
        }
    }

    private func handle(_ connection: NWConnection, method: String, path: String, headers: [String: String], body: Data) {
        guard !rejectAuthentication,
              headers["authorization"] == "Basic " + Data("fixture:fixture-only".utf8).base64EncodedString()
        else { respond(connection, status: 401); return }
        guard path.hasPrefix("/remote.php/dav/files/fixture/") else { respond(connection, status: 404); return }
        switch method {
        case "PROPFIND":
            propfinds += 1
            if let onHeldPROPFIND {
                self.onHeldPROPFIND = nil
                heldPROPFIND = (connection, path)
                onHeldPROPFIND()
            } else { propfind(connection, path: path) }
        case "GET":
            gets += 1
            if let onHeldGET {
                self.onHeldGET = nil
                heldGET = (connection, path)
                onHeldGET()
            } else { get(connection, path: path) }
        case "MKCOL":
            collections += 1
            collectionExists = true
            respond(connection, status: 201)
        case "PUT":
            puts += 1
            guard collectionExists else { respond(connection, status: 409); return }
            let conflict = conflictNextPUT || (headers["if-match"] != nil && headers["if-match"] != etag)
                || (headers["if-none-match"] == "*" && files[path] != nil)
            if conflict {
                conflictNextPUT = false
                conflicts += 1
                respond(connection, status: 412)
            } else {
                files[path] = body
                version += 1
                respond(connection, status: 204)
            }
        default: respond(connection, status: 405)
        }
    }

    private func propfind(_ connection: NWConnection, path: String) {
        let accountRoot = "/remote.php/dav/files/fixture/"
        if path == accountRoot || collectionExists { respond(connection, status: 207) }
        else { respond(connection, status: 404) }
    }

    private func get(_ connection: NWConnection, path: String) {
        if let body = files[path] { respond(connection, status: 200, body: body) }
        else { respond(connection, status: 404) }
    }

    private func respond(_ connection: NWConnection, status: Int, body: Data = Data()) {
        var response = Data("HTTP/1.1 \(status) Fixture\r\nConnection: close\r\nOC-ETag: \(etag)\r\nContent-Length: \(body.count)\r\n\r\n".utf8)
        response.append(body)
        connection.send(content: response, completion: .contentProcessed { [weak self] _ in
            Task { @MainActor in self?.close(connection) }
        })
    }

    private func close(_ connection: NWConnection) {
        connection.cancel()
        connections.removeValue(forKey: ObjectIdentifier(connection))
    }
}
