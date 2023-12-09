import { resolve } from "node:path";

import autoprefixer from "autoprefixer";
import cssnano from "cssnano";
import { glob } from "glob";
import MiniCssExtractPlugin from "mini-css-extract-plugin";
import { PurgeCSSPlugin } from "purgecss-webpack-plugin";
import TerserPlugin from "terser-webpack-plugin";
import { Configuration } from "webpack";

const PATHS = { views: resolve(__dirname, "..", "views") };

const collectWhitelist = () => {
    return [
        "toggled", "collapsed", "collapsing", "show", "modal-backdrop", "modal-open", "d-none", "border-dark",
    ];
};

const config: Configuration = {
    entry: {
        bundle: "./src/index.js",
        "font-roboto-mono": "./src/font-roboto-mono.scss",
    },
    output: {
        clean: true,
        filename: "./js/[name].min.js",
        library: { type: "umd" },
    },
    plugins: [
        new MiniCssExtractPlugin({ filename: "./css/[name].min.css" }),
        new PurgeCSSPlugin({
            paths: glob.sync(`${PATHS.views}/**/*`, { nodir: true }),
            safelist() {
                return {
                    standard: collectWhitelist(),
                    deep: [],
                    greedy: [],
                };
            },
            blocklist() {
                return [];
            },
        }),
    ],
    module: {
        rules: [
            {
                test: /\.ts$/i,
                use: [
                    {
                        loader: "babel-loader",
                        options: { presets: ["@babel/preset-env", "@babel/preset-typescript"] },
                    },
                ],
            },
            {
                test: /\.js$/i,
                use: [
                    {
                        loader: "babel-loader",
                        options: { presets: ["@babel/preset-env"] },
                    },
                ],
            },
            {
                test: /\.(sa|sc|c)ss$/i,
                use: [
                    MiniCssExtractPlugin.loader,
                    "css-loader",
                    {
                        loader: "postcss-loader",
                        options: {
                            postcssOptions: {
                                plugins: [
                                    autoprefixer,
                                    cssnano({ preset: ["default", { discardComments: { removeAll: true } }] }),
                                ],
                            },
                        },
                    },
                    "sass-loader",
                ],
            },
            {
                test: /\.(eot|woff|woff2|[ot]tf)$/,
                type: "asset/resource",
                generator: { filename: "fonts/[name][ext]" },
            },
            {
                test: /.*font.*\.svg$/,
                type: "asset/resource",
                generator: { filename: "fonts/[name][ext]" },
            },
        ],
    },
    resolve: { extensionAlias: { ".js": [".ts", ".js"] } },
    optimization: {
        minimizer: [
            new TerserPlugin({
                extractComments: false,
                terserOptions: { format: { comments: false } },
            }),
        ],
    },
};

export default config;
