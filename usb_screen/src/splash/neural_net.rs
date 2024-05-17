//创建一个前馈神经网络

use alloc::vec::Vec;
use embassy_rp::clocks::RoscRng;
use micromath::F32Ext;
use super::utils::random_clamped;
use super::params::{ ACTIVATION_RESPONSE,BIAS,NEURONS_PER_HIDDEN_LAYER, NUM_HIDDEN,NUM_OUTPUTS,NUM_INPUTS, };

//神经元
pub struct Neuron{
    //进入神经细胞的输入个数
    num_inputs: usize,
    //为每个输入提供的权重
    weight: Vec<f32>,
}

impl Clone for Neuron {
    fn clone(&self) -> Neuron {
        Neuron{ num_inputs: self.num_inputs, weight: self.weight.clone() }
    }

    fn clone_from(&mut self, source: &Self) {
        self.num_inputs = source.num_inputs;
        self.weight = source.weight.clone();
    }
}

impl Neuron{
    pub fn new(num_inputs: usize, rng: &mut RoscRng) -> Neuron{
        //我们需要一个额外的权重，因此+1
        let mut weight = Vec::new();
        //用初始随机值设置权重
        for _ in 0..num_inputs+1 {
            let _ = weight.push(random_clamped(rng));
        }
        Neuron{
            num_inputs: num_inputs+1 as usize,
            weight: weight
        }
    }
}

struct NeuronLayer{
    //本层使用的神经细胞数目
    num_neurons: usize,
    //神经细胞的层
    neurons: Vec<Neuron>,
}

impl NeuronLayer{
    //创建所需大小的神经元层(神经元数量, 每个神经元输入数)
    pub fn new(num_neurons: usize, num_inputs_per_neuron: usize, rng: &mut RoscRng) -> NeuronLayer{
        let mut neurons = Vec::new();
        for _ in 0..num_neurons {
            let _ = neurons.push(Neuron::new(num_inputs_per_neuron, rng));
        }
        NeuronLayer{
            num_neurons: num_neurons,
            neurons: neurons
        }
    }
}

pub struct NeuralNet{
    num_inputs: usize,
    //num_outputs: usize,
    num_hidden_layers: usize,
    //neurons_per_hidden_lyr: usize,
    //存储用于包括输出层的每层神经元
    layers: Vec<NeuronLayer>
}

impl NeuralNet{
    pub fn new(rng: &mut RoscRng) ->NeuralNet {
        let mut layers = Vec::new();
        if NUM_HIDDEN>0 {
            //构建ANN。 权重最初都被设置为随机值-1 <w <1
            //创建第一个隐藏层: 
            //神经元个数=NEURONS_PER_HIDDEN_LAYER, 输入数=NumInputs
            let _ = layers.push(NeuronLayer::new(NEURONS_PER_HIDDEN_LAYER, NUM_INPUTS, rng));
            for _ in 0..NUM_HIDDEN-1 {
                //中间层: 神经元个数=NEURONS_PER_HIDDEN_LAYER, 输入数=(衔接第一个隐藏层的输出，即：[第一个]隐藏层的神经元个数)
                let _ = layers.push(NeuronLayer::new(NEURONS_PER_HIDDEN_LAYER, NEURONS_PER_HIDDEN_LAYER, rng));
            }
            //创建输出层: 神经元个数=NumOutputs（即输出数）, 输入数=(衔接第隐藏层的输出，即：隐藏层的神经元个数)
            let _ = layers.push(NeuronLayer::new(NUM_OUTPUTS, NEURONS_PER_HIDDEN_LAYER, rng));

        }else{//无隐藏层时，只需创建输出层
            //创建输出层: 只有一层的神经元数量=输出数
            let _ = layers.push(NeuronLayer::new(NUM_OUTPUTS, NUM_INPUTS, rng));
        }
        NeuralNet{
            num_inputs: NUM_INPUTS,
            //num_outputs: NUM_OUTPUTS,
            num_hidden_layers: NUM_HIDDEN,
            //neurons_per_hidden_lyr: NEURONS_PER_HIDDEN_LAYER,
            layers: layers,
        }
    }

    //给定一个双精度Vec，该函数用新值替换NN中的权重
    //当遗传算法完成一代时，新一代的权重必须重新插入神经网络
    pub fn put_weights(&mut self, weights: &Vec<f32>){
        let mut weight = 0;
        //循环每一层
        for i in 0..self.num_hidden_layers+1 {
            //循环每个神经元
            for j in 0..self.layers[i].num_neurons {
                //循环每个权重
                for k in 0..self.layers[i].neurons[j].num_inputs {
                   self.layers[i].neurons[j].weight[k] = weights[weight];
                   weight += 1;
                }
            }
        }
    }

    //给定输入向量该函数计算输出向量
    //update函数可称为神经网络的“主要动力”。这里，输入网络的数据input是以双精度Vector的数据格式传递进来的。
    //update函数通过对每个层的循环来处理 输入x权重的相乘与求和，再以所得的和数作为激励值，通过S形函数来计算出
    //每个神经细胞的输出。update函数返回的也是一个双精度Vector，它对应的就是人工神经网络的所有输出
    pub fn update(&self, inputs: &mut Vec<f32>) -> Vec<f32> {
        //let mut inputs = _inputs;
        //存储每个层的结果输出
        let mut outputs = Vec::new();
        //let mut weight = 0;
        
        //首先检查我们有正确的输入量
        if inputs.len()!=self.num_inputs{
            //不过不正确，就返回一个空的vector
            return outputs;
        }

        //循环每一层
        for i in 0..self.num_hidden_layers+1 {
            if i>0 {
                *inputs = outputs.clone();
            }
            outputs.clear();
            let mut weight = 0;
            //对每个神经细胞，求 输入x对应权重乘积的综合。并将总和赋值给S形函数以计算输出
            for j in 0..self.layers[i].num_neurons {
                let mut netinput = 0.0;
                let num_inputs = self.layers[i].neurons[j].num_inputs;
                //循环每个权重
                for k in 0..num_inputs-1 {
                    //计算权重x输入乘积的总和
                    netinput += self.layers[i].neurons[j].weight[k] * inputs[weight];
                    weight += 1;
                }
                //加入偏移值
                //每个神经细胞的权重vector的最后一个权重实际是偏移值，一般将它设置为-1。
                //可以改变它，考察它对创建的网络的功能有什么影响，这个值通常是不应该改变的。
                netinput += self.layers[i].neurons[j].weight[num_inputs-1] * BIAS;
                //每一层的输出产生之后，就应将他们保存起来
                //但用求和函数累加在一起的激励总值首先要通过S形函数的过滤，才能得到输出
                let _ = outputs.push(NeuralNet::sigmoid(netinput, ACTIVATION_RESPONSE));
                weight = 0;
            }
        }
        outputs
    }

    //S形响应曲线
    //当已知神经细胞所有输入x权重的乘积之和时，这一方法将它送入S形的激励函数
    pub fn sigmoid(netinput: f32, response: f32) -> f32{
        1.0/(1.0+(-netinput/response).exp())
    }
}