const {resolve} = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = resolve(__dirname, "dist");
const assets = resolve(__dirname, "..", "game", "src", "assets");

module.exports = {
    mode: "production",
    entry: {
        index: "./src/index.js",
    },
    output: {
        path: dist,
        filename: "bundle.js",
    },
    devServer: {
        contentBase: dist,
    },
    plugins: [
        new CopyPlugin({
            patterns: [assets],
        }),
        new WasmPackPlugin({
            crateDirectory: __dirname,
        }),
        new HtmlWebpackPlugin(),
    ],
    experiments: {
        asyncWebAssembly: true,
        syncWebAssembly: true,
    }
};
