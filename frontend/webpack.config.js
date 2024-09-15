const webpack = require('webpack');
const path = require('path');
require('dotenv').config({ path: path.resolve(__dirname, '..', '.env') });

const HtmlWebpackPlugin = require('html-webpack-plugin');

const env_config = {
    development: {
        devtool: 'inline-source-map',
    },
    production: {
        devtool: 'source-map',
    }
}

module.exports = (_env, argv) => {
    return {
        mode: argv.mode,
        devtool: env_config[argv.mode].devtool,
        entry: './src/index.ts',
        output: {
            filename: 'main-[hash].js',
            path: path.resolve(__dirname, 'dist')
        },
        module: {
            rules: [
                {
                    test: /\.ts$/,
                    use: 'ts-loader',
                    exclude: /node_modules/,
                },
            ],
        },
        resolve: {
            extensions: ['.ts', '.js'],
        },
        experiments: {
            asyncWebAssembly: true,
            syncWebAssembly: true
        },
        plugins: [
            new webpack.EnvironmentPlugin(['CLIENT_ID']),
            new HtmlWebpackPlugin({
                title: 'Silly Sync',
                template: './src/index.html'
            })
        ],
        devServer: {
            static: {
                directory: path.join(__dirname, 'dist'),
            },
            compress: true,
            allowedHosts: 'all',
            port: 3000,
            proxy: [
                {
                    context: ['/api'],
                    target: 'http://localhost:8000',
                    pathRewrite: { '^/api': '' },
                    ws: true
                },
            ],
        }
    }
}