const std = @import("std");
const net = std.net;
const posix = std.posix;

pub fn main() !void {
    const address = try std.net.Address.parseIp("0.0.0.0", 34254);

    const tpe: u32 = posix.SOCK.STREAM;
    const protocol = posix.IPPROTO.TCP;
    const listener = try posix.socket(address.any.family, tpe, protocol);
    defer posix.close(listener);

    try posix.setsockopt(listener, posix.SOL.SOCKET, posix.SO.REUSEADDR, &std.mem.toBytes(@as(c_int, 1)));
    try posix.bind(listener, &address.any, address.getOsSockLen());
    try posix.listen(listener, 128);
    var buf: [1024]u8 = undefined;
    var client_address: net.Address = undefined;
    var client_address_len: posix.socklen_t = @sizeOf(net.Address);

    const socket = try posix.accept(listener, &client_address.any, &client_address_len, 0);
    defer posix.close(socket);

    std.debug.print("{} connected\n", .{client_address});
    while (true) {

        const read = posix.read(socket, &buf) catch |err| {
            std.debug.print("error reading: {}\n", .{err});
            continue;
        };
        std.debug.print("{s}", .{buf});
        // Clear buffer
        // For now we are just printing this out. The goal is to pass the contents of the buffer to wayland clipboard
        buf = undefined;

        if (read == 0) {
            continue;
        }

    }
}

fn write(socket: posix.socket_t, msg: []const u8) !void {
    var pos: usize = 0;
    while (pos < msg.len) {
        const written = try posix.write(socket, msg[pos..]);
        if (written == 0) {
            return error.Closed;
        }
        pos += written;
    }
}
